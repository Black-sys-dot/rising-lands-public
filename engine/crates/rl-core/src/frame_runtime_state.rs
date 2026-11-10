//! Semantic live task/group/action state with explicit original layout bindings.
//! Unknown bytes remain in the world's projection template. No packed record
//! or synthetic initialization replaces the captured allocator/list state.
use crate::{EngineError, MemoriaJogo, Result};
const TASK_POOL: u32 = 0x512320;
const GROUP_POOL: u32 = 0x51a9e0;
const FREE: u32 = 0x5148a0;
const CONTEXTS: u32 = 0x5148b0;
pub const TASK_COUNT: usize = 100;
pub const GROUP_COUNT: usize = 100;
fn link(value: u32) -> Option<u32> {
    if value == u32::MAX {
        None
    } else {
        Some(value)
    }
}
fn raw(value: Option<u32>) -> u32 {
    value.unwrap_or(u32::MAX)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskState {
    pub next_task: Option<u32>,
    pub parent_task: Option<u32>,
    pub instruction: u32,
    pub wait_ticks: i32,
    pub locals: [u32; 15],
    pub result: u32,
    pub child_kind: u16,
    pub kind: u16,
    pub player: u8,
    pub sleeping: u8,
    pub return_pending: u8,
}
impl TaskState {
    fn load(m: &MemoriaJogo, p: u32) -> Result<Self> {
        let mut locals = [0; 15];
        for (i, v) in locals.iter_mut().enumerate() {
            *v = m.u32(p + 0x18 + 4 * i as u32)?;
        }
        Ok(Self {
            next_task: link(m.u32(p)?),
            parent_task: link(m.u32(p + 4)?),
            instruction: m.u32(p + 12)?,
            wait_ticks: m.u32(p + 0x14)? as i32,
            locals,
            result: m.u32(p + 0x54)?,
            child_kind: m.u16(p + 0x58)?,
            kind: m.u16(p + 0x5a)?,
            player: m.u8(p + 0x5c)?,
            sleeping: m.u8(p + 0x5e)?,
            return_pending: m.u8(p + 0x5f)?,
        })
    }
    fn project(&self, m: &mut MemoriaJogo, p: u32) -> Result<()> {
        m.set_u32(p, raw(self.next_task))?;
        m.set_u32(p + 4, raw(self.parent_task))?;
        m.set_u32(p + 12, self.instruction)?;
        m.set_u32(p + 0x14, self.wait_ticks as u32)?;
        for (i, v) in self.locals.iter().enumerate() {
            m.set_u32(p + 0x18 + 4 * i as u32, *v)?;
        }
        m.set_u32(p + 0x54, self.result)?;
        m.set_u16(p + 0x58, self.child_kind)?;
        m.set_u16(p + 0x5a, self.kind)?;
        m.set_u8(p + 0x5c, self.player)?;
        m.set_u8(p + 0x5e, self.sleeping)?;
        m.set_u8(p + 0x5f, self.return_pending)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupState {
    pub next_group: Option<u32>,
    pub previous_group: Option<u32>,
    pub waiting_task: Option<u32>,
    pub battle: Option<u32>,
    pub first_unit: Option<u16>,
    pub unit_count: u16,
    pub active: u8,
    pub kind: u8,
    pub behaviour: u8,
}
impl GroupState {
    fn load(m: &MemoriaJogo, p: u32) -> Result<Self> {
        let first = m.u16(p + 16)?;
        Ok(Self {
            next_group: link(m.u32(p)?),
            previous_group: link(m.u32(p + 4)?),
            waiting_task: link(m.u32(p + 8)?),
            battle: link(m.u32(p + 12)?),
            first_unit: if first == u16::MAX { None } else { Some(first) },
            unit_count: m.u16(p + 18)?,
            active: m.u8(p + 24)?,
            kind: m.u8(p + 25)?,
            behaviour: m.u8(p + 27)?,
        })
    }
    fn project(&self, m: &mut MemoriaJogo, p: u32) -> Result<()> {
        m.set_u32(p, raw(self.next_group))?;
        m.set_u32(p + 4, raw(self.previous_group))?;
        m.set_u32(p + 8, raw(self.waiting_task))?;
        m.set_u32(p + 12, raw(self.battle))?;
        m.set_u16(p + 16, self.first_unit.unwrap_or(u16::MAX))?;
        m.set_u16(p + 18, self.unit_count)?;
        m.set_u8(p + 24, self.active)?;
        m.set_u8(p + 25, self.kind)?;
        m.set_u8(p + 27, self.behaviour)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskListHead {
    pub first: Option<u32>,
    pub count: u32,
}
impl TaskListHead {
    fn load(m: &MemoriaJogo, p: u32) -> Result<Self> {
        Ok(Self {
            first: link(m.u32(p)?),
            count: m.u32(p + 4)?,
        })
    }
    fn project(&self, m: &mut MemoriaJogo, p: u32) -> Result<()> {
        m.set_u32(p, raw(self.first))?;
        m.set_u32(p + 4, self.count)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeState {
    pub tasks: Vec<TaskState>,
    pub groups: Vec<GroupState>,
    pub free_tasks: TaskListHead,
    pub contexts: [TaskListHead; 5],
    pub active_tasks: u32,
    pub group_cursor: u32,
    /// Five players, twenty-four group kinds each, in original iteration order.
    pub group_heads: Vec<Option<u32>>,
    pub iterator_group: u32,
    pub iterator_unit: Option<u16>,
    pub action_count: u32,
    pub pending_groups: [Option<u32>; 16],
    pub pending_count: u8,
    pub live_monsters: u32,
    list_bindings: [[u32; 2]; 6],
}
impl RuntimeState {
    fn layout(m: &MemoriaJogo) -> Result<bool> {
        for (address, size) in [
            (TASK_POOL, 9600),
            (FREE, 16),
            (CONTEXTS, 80),
            (0x514904, 4),
            (GROUP_POOL, 3200),
            (0x51b660, 4),
            (0x51b664, 480),
            (0x51b844, 6),
            (0x50ae80, 4),
            (0x50b134, 1),
            (0x50b138, 64),
            (0x50b178, 4),
        ] {
            match m.read(address, size) {
                Ok(_) => {}
                Err(EngineError::UndeclaredAccess { .. }) => return Ok(false),
                Err(e) => return Err(e),
            }
        }
        Ok(true)
    }
    fn bindings(m: &MemoriaJogo) -> Result<[[u32; 2]; 6]> {
        let mut result = [[0; 2]; 6];
        for (i, v) in result.iter_mut().enumerate() {
            let p = if i == 0 {
                FREE
            } else {
                CONTEXTS + (i as u32 - 1) * 16
            };
            *v = [m.u32(p + 8)?, m.u32(p + 12)?];
        }
        Ok(result)
    }
    pub fn load(m: &MemoriaJogo) -> Result<Option<Self>> {
        if !Self::layout(m)? {
            return Ok(None);
        }
        let tasks = (0..TASK_COUNT)
            .map(|i| TaskState::load(m, TASK_POOL + 96 * i as u32))
            .collect::<Result<Vec<_>>>()?;
        let groups = (0..GROUP_COUNT)
            .map(|i| GroupState::load(m, GROUP_POOL + 32 * i as u32))
            .collect::<Result<Vec<_>>>()?;
        let contexts = (0..5)
            .map(|i| TaskListHead::load(m, CONTEXTS + 16 * i))
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .expect("five contexts");
        let group_heads = (0..120)
            .map(|i| m.u32(0x51b664 + 4 * i).map(link))
            .collect::<Result<Vec<_>>>()?;
        let mut pending_groups = [None; 16];
        for (i, v) in pending_groups.iter_mut().enumerate() {
            *v = link(m.u32(0x50b138 + 4 * i as u32)?);
        }
        let iterator = m.u16(0x51b848)?;
        Ok(Some(Self {
            tasks,
            groups,
            free_tasks: TaskListHead::load(m, FREE)?,
            contexts,
            active_tasks: m.u32(0x514904)?,
            group_cursor: m.u32(0x51b660)?,
            group_heads,
            iterator_group: m.u32(0x51b844)?,
            iterator_unit: if iterator == u16::MAX {
                None
            } else {
                Some(iterator)
            },
            action_count: m.u32(0x50ae80)?,
            pending_groups,
            pending_count: m.u8(0x50b134)?,
            live_monsters: m.u32(0x50b178)?,
            list_bindings: Self::bindings(m)?,
        }))
    }
    pub(crate) fn reload(&mut self, m: &MemoriaJogo) -> Result<()> {
        if !Self::layout(m)? {
            return Err(EngineError::ProjectionFieldMismatch { address: TASK_POOL });
        }
        for (i, value) in self.tasks.iter_mut().enumerate() {
            *value = TaskState::load(m, TASK_POOL + 96 * i as u32)?;
        }
        for (i, value) in self.groups.iter_mut().enumerate() {
            *value = GroupState::load(m, GROUP_POOL + 32 * i as u32)?;
        }
        self.free_tasks = TaskListHead::load(m, FREE)?;
        for (i, value) in self.contexts.iter_mut().enumerate() {
            *value = TaskListHead::load(m, CONTEXTS + 16 * i as u32)?;
        }
        self.active_tasks = m.u32(0x514904)?;
        self.group_cursor = m.u32(0x51b660)?;
        for (i, value) in self.group_heads.iter_mut().enumerate() {
            *value = link(m.u32(0x51b664 + 4 * i as u32)?);
        }
        self.iterator_group = m.u32(0x51b844)?;
        self.iterator_unit = match m.u16(0x51b848)? {
            65535 => None,
            v => Some(v),
        };
        self.action_count = m.u32(0x50ae80)?;
        for (i, value) in self.pending_groups.iter_mut().enumerate() {
            *value = link(m.u32(0x50b138 + 4 * i as u32)?);
        }
        self.pending_count = m.u8(0x50b134)?;
        self.live_monsters = m.u32(0x50b178)?;
        self.list_bindings = Self::bindings(m)?;
        Ok(())
    }
    pub fn project(&self, m: &mut MemoriaJogo) -> Result<()> {
        if self.tasks.len() != TASK_COUNT
            || self.groups.len() != GROUP_COUNT
            || self.group_heads.len() != 120
            || !Self::layout(m)?
            || self.list_bindings != Self::bindings(m)?
        {
            return Err(EngineError::ProjectionFieldMismatch { address: TASK_POOL });
        }
        for (i, t) in self.tasks.iter().enumerate() {
            t.project(m, TASK_POOL + 96 * i as u32)?;
        }
        for (i, g) in self.groups.iter().enumerate() {
            g.project(m, GROUP_POOL + 32 * i as u32)?;
        }
        self.free_tasks.project(m, FREE)?;
        for (i, c) in self.contexts.iter().enumerate() {
            c.project(m, CONTEXTS + 16 * i as u32)?;
        }
        m.set_u32(0x514904, self.active_tasks)?;
        m.set_u32(0x51b660, self.group_cursor)?;
        for (i, h) in self.group_heads.iter().enumerate() {
            m.set_u32(0x51b664 + 4 * i as u32, raw(*h))?;
        }
        m.set_u32(0x51b844, self.iterator_group)?;
        m.set_u16(0x51b848, self.iterator_unit.unwrap_or(u16::MAX))?;
        m.set_u32(0x50ae80, self.action_count)?;
        for (i, g) in self.pending_groups.iter().enumerate() {
            m.set_u32(0x50b138 + 4 * i as u32, raw(*g))?;
        }
        m.set_u8(0x50b134, self.pending_count)?;
        m.set_u32(0x50b178, self.live_monsters)
    }
}
