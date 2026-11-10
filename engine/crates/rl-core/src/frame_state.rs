//! Incremental semantic frame state and explicit original-layout bindings.
//!
//! Providers execute against a checked projected transaction. Play mode refreshes
//! only changed semantic records after success; audit mode can reload all values. Unmodeled record bytes stay in Projection's
//! template; they never become zero-filled or invented Rust state.
use crate::{EngineError, MemoriaJogo, Result};

pub const UNIT_COUNT: usize = 800;
pub const STRUCTURE_COUNT: usize = 1000;
const STRIDE: u32 = 276;
const UNIT_POOL_POINTER: u32 = 0x4e19a0;
const STRUCTURE_POOL: u32 = 0x49a0c8;

fn r8(r: &[u8], o: usize) -> u8 {
    r[o]
}
fn r16(r: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(r[o..o + 2].try_into().expect("record word"))
}
fn r32(r: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(r[o..o + 4].try_into().expect("record dword"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitState {
    pub owner: u8,
    pub kind: u8,
    pub state: u8,
    pub provisions: u16,
    pub cell: u16,
    pub on_map: u8,
    pub fatigue_ticks: u16,
    pub previous_cell: u16,
    pub destination_cell: u16,
    pub moving: u8,
    pub turning: u8,
    pub order: u8,
    pub attack_phase: u8,
    pub facing: u8,
    pub target_facing: u8,
    pub airborne: u8,
    pub attack_range: u8,
    pub vision_radius: u8,
    pub special_direction: u8,
    pub status_flags: u8,
    pub fatigue_bonus: u8,
    pub frame_flag: u8,
    pub collision_cell_0: u16,
    pub collision_cell_1: u16,
    pub collision_cell_2: u16,
    pub collision_cell_3: u16,
    pub reload_ticks: u16,
    pub sound_handle: u32,
    pub next_group_unit: u16,
    pub observed_state: u8,
    pub observed_order: u8,
    pub group: u32,
    pub path_mode: u8,
    pub path_wait_ticks: u8,
    pub turn_ticks: u8,
    pub turn_period: u8,
    pub step_progress: u8,
    pub step_speed: u8,
    pub walk_frame: u8,
    pub walk_frame_count: u8,
    pub walk_frame_period: u8,
    pub walk_frame_ticks: u8,
    pub order_cell: u16,
    pub attack_kind: u8,
    pub attack_reference: u16,
    pub work_structure: u16,
}
impl UnitState {
    fn load(m: &MemoriaJogo, base: u32) -> Result<Self> {
        let record = m.read(base, 276)?;
        Ok(Self {
            owner: r8(record, 0x0),
            kind: r8(record, 0x1),
            state: r8(record, 0x2),
            provisions: r16(record, 0x4),
            cell: r16(record, 0x8),
            on_map: r8(record, 0x72),
            fatigue_ticks: r16(record, 0x74),
            previous_cell: r16(record, 0x7c),
            destination_cell: r16(record, 0x7e),
            moving: r8(record, 0x86),
            turning: r8(record, 0x87),
            order: r8(record, 0x89),
            attack_phase: r8(record, 0x8a),
            facing: r8(record, 0x90),
            target_facing: r8(record, 0x91),
            airborne: r8(record, 0xab),
            attack_range: r8(record, 0xb0),
            vision_radius: r8(record, 0xbb),
            special_direction: r8(record, 0xda),
            status_flags: r8(record, 0xdc),
            fatigue_bonus: r8(record, 0xf8),
            frame_flag: r8(record, 0xf9),
            collision_cell_0: r16(record, 0xfa),
            collision_cell_1: r16(record, 0xfc),
            collision_cell_2: r16(record, 0xfe),
            collision_cell_3: r16(record, 0x100),
            reload_ticks: r16(record, 0x102),
            sound_handle: r32(record, 0x104),
            next_group_unit: r16(record, 0x108),
            observed_state: r8(record, 0x10a),
            observed_order: r8(record, 0x10b),
            group: r32(record, 0x10c),
            path_mode: r8(record, 0x88),
            path_wait_ticks: r8(record, 0x8b),
            turn_ticks: r8(record, 0x92),
            turn_period: r8(record, 0x93),
            step_progress: r8(record, 0x97),
            step_speed: r8(record, 0x98),
            walk_frame: r8(record, 0xa4),
            walk_frame_count: r8(record, 0xa5),
            walk_frame_period: r8(record, 0xa7),
            walk_frame_ticks: r8(record, 0xa9),
            order_cell: r16(record, 0xb2),
            attack_kind: r8(record, 0xb7),
            attack_reference: r16(record, 0xb8),
            work_structure: r16(record, 0xc0),
        })
    }
    fn project(&self, m: &mut MemoriaJogo, base: u32) -> Result<()> {
        m.set_u8(base.wrapping_add(0x0), self.owner)?;
        m.set_u8(base.wrapping_add(0x1), self.kind)?;
        m.set_u8(base.wrapping_add(0x2), self.state)?;
        m.set_u16(base.wrapping_add(0x4), self.provisions)?;
        m.set_u16(base.wrapping_add(0x8), self.cell)?;
        m.set_u8(base.wrapping_add(0x72), self.on_map)?;
        m.set_u16(base.wrapping_add(0x74), self.fatigue_ticks)?;
        m.set_u16(base.wrapping_add(0x7c), self.previous_cell)?;
        m.set_u16(base.wrapping_add(0x7e), self.destination_cell)?;
        m.set_u8(base.wrapping_add(0x86), self.moving)?;
        m.set_u8(base.wrapping_add(0x87), self.turning)?;
        m.set_u8(base.wrapping_add(0x89), self.order)?;
        m.set_u8(base.wrapping_add(0x8a), self.attack_phase)?;
        m.set_u8(base.wrapping_add(0x90), self.facing)?;
        m.set_u8(base.wrapping_add(0x91), self.target_facing)?;
        m.set_u8(base.wrapping_add(0xab), self.airborne)?;
        m.set_u8(base.wrapping_add(0xb0), self.attack_range)?;
        m.set_u8(base.wrapping_add(0xbb), self.vision_radius)?;
        m.set_u8(base.wrapping_add(0xda), self.special_direction)?;
        m.set_u8(base.wrapping_add(0xdc), self.status_flags)?;
        m.set_u8(base.wrapping_add(0xf8), self.fatigue_bonus)?;
        m.set_u8(base.wrapping_add(0xf9), self.frame_flag)?;
        m.set_u16(base.wrapping_add(0xfa), self.collision_cell_0)?;
        m.set_u16(base.wrapping_add(0xfc), self.collision_cell_1)?;
        m.set_u16(base.wrapping_add(0xfe), self.collision_cell_2)?;
        m.set_u16(base.wrapping_add(0x100), self.collision_cell_3)?;
        m.set_u16(base.wrapping_add(0x102), self.reload_ticks)?;
        m.set_u32(base.wrapping_add(0x104), self.sound_handle)?;
        m.set_u16(base.wrapping_add(0x108), self.next_group_unit)?;
        m.set_u8(base.wrapping_add(0x10a), self.observed_state)?;
        m.set_u8(base.wrapping_add(0x10b), self.observed_order)?;
        m.set_u32(base.wrapping_add(0x10c), self.group)?;
        m.set_u8(base.wrapping_add(0x88), self.path_mode)?;
        m.set_u8(base.wrapping_add(0x8b), self.path_wait_ticks)?;
        m.set_u8(base.wrapping_add(0x92), self.turn_ticks)?;
        m.set_u8(base.wrapping_add(0x93), self.turn_period)?;
        m.set_u8(base.wrapping_add(0x97), self.step_progress)?;
        m.set_u8(base.wrapping_add(0x98), self.step_speed)?;
        m.set_u8(base.wrapping_add(0xa4), self.walk_frame)?;
        m.set_u8(base.wrapping_add(0xa5), self.walk_frame_count)?;
        m.set_u8(base.wrapping_add(0xa7), self.walk_frame_period)?;
        m.set_u8(base.wrapping_add(0xa9), self.walk_frame_ticks)?;
        m.set_u16(base.wrapping_add(0xb2), self.order_cell)?;
        m.set_u8(base.wrapping_add(0xb7), self.attack_kind)?;
        m.set_u16(base.wrapping_add(0xb8), self.attack_reference)?;
        m.set_u16(base.wrapping_add(0xc0), self.work_structure)?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureState {
    pub origin_cell: u16,
    pub owner: u8,
    pub state: u8,
    pub kind: u8,
    pub production_enabled: u8,
    pub requested_kind: u8,
    pub production_progress: u32,
    pub illuminated_player_0: u8,
    pub illuminated_player_1: u8,
    pub illuminated_player_2: u8,
    pub illuminated_player_3: u8,
}
impl StructureState {
    fn load(m: &MemoriaJogo, base: u32) -> Result<Self> {
        let record = m.read(base, 276)?;
        Ok(Self {
            origin_cell: r16(record, 0x2),
            owner: r8(record, 0x4),
            state: r8(record, 0x5),
            kind: r8(record, 0x8),
            production_enabled: r8(record, 0x74),
            requested_kind: r8(record, 0x7b),
            production_progress: r32(record, 0x8c),
            illuminated_player_0: r8(record, 0x110),
            illuminated_player_1: r8(record, 0x111),
            illuminated_player_2: r8(record, 0x112),
            illuminated_player_3: r8(record, 0x113),
        })
    }
    fn project(&self, m: &mut MemoriaJogo, base: u32) -> Result<()> {
        m.set_u16(base.wrapping_add(0x2), self.origin_cell)?;
        m.set_u8(base.wrapping_add(0x4), self.owner)?;
        m.set_u8(base.wrapping_add(0x5), self.state)?;
        m.set_u8(base.wrapping_add(0x8), self.kind)?;
        m.set_u8(base.wrapping_add(0x74), self.production_enabled)?;
        m.set_u8(base.wrapping_add(0x7b), self.requested_kind)?;
        m.set_u32(base.wrapping_add(0x8c), self.production_progress)?;
        m.set_u8(base.wrapping_add(0x110), self.illuminated_player_0)?;
        m.set_u8(base.wrapping_add(0x111), self.illuminated_player_1)?;
        m.set_u8(base.wrapping_add(0x112), self.illuminated_player_2)?;
        m.set_u8(base.wrapping_add(0x113), self.illuminated_player_3)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameGlobals {
    pub game_frame: u32,
    pub mission: u8,
    pub ai_rng_seed: u32,
    pub end_condition: u8,
    pub hud_refresh: u8,
    pub end_condition_ticks: u32,
    pub network: u8,
    pub permutation_cursor: u8,
    pub input_player: u8,
    pub music_disabled: u8,
    pub game_state: u8,
    pub local_player: u8,
    pub view_x: u16,
    pub view_y: u16,
    pub view_width: u16,
    pub view_height: u16,
    pub iteration: u32,
    pub iteration_secondary: u32,
    pub iteration_tertiary: u32,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub frame_exit: u8,
    pub unit_counts: [u16; 5],
    pub structure_counts: [u32; 4],
    pub local_unit_count: u16,
    pub other_unit_count: u16,
    pub unit_kind_counts: [[u16; 26]; 4],
}
impl FrameGlobals {
    fn load(m: &MemoriaJogo) -> Result<Self> {
        let mut unit_counts = [0u16; 5];
        for (i, value) in unit_counts.iter_mut().enumerate() {
            *value = m.u16(0x4e1928 + 2 * i as u32)?;
        }
        let mut structure_counts = [0u32; 4];
        for (i, value) in structure_counts.iter_mut().enumerate() {
            *value = m.u32(0x4dfaa8 + 4 * i as u32)?;
        }
        let mut unit_kind_counts = [[0u16; 26]; 4];
        for (owner, kinds) in unit_kind_counts.iter_mut().enumerate() {
            for (kind, value) in kinds.iter_mut().enumerate() {
                *value = m.u16(0x4e1a3c + owner as u32 * 0x34 + kind as u32 * 2)?;
            }
        }
        Ok(Self {
            game_frame: m.u32(0x496d40)?,
            mission: m.u8(0x496fe4)?,
            ai_rng_seed: m.u32(0x498db4)?,
            end_condition: m.u8(0x4e05a0)?,
            hud_refresh: m.u8(0x4e05a2)?,
            end_condition_ticks: m.u32(0x4e061c)?,
            network: m.u8(0x4e0730)?,
            permutation_cursor: m.u8(0x4e1d18)?,
            input_player: m.u8(0x4e2480)?,
            music_disabled: m.u8(0x4e2f28)?,
            game_state: m.u8(0x4e2f78)?,
            local_player: m.u8(0x508041)?,
            view_x: m.u16(0x508720)?,
            view_y: m.u16(0x508722)?,
            view_width: m.u16(0x508724)?,
            view_height: m.u16(0x508726)?,
            iteration: m.u32(0x508f7c)?,
            iteration_secondary: m.u32(0x508f80)?,
            iteration_tertiary: m.u32(0x508f84)?,
            cursor_x: m.u16(0x5099a8)?,
            cursor_y: m.u16(0x5099aa)?,
            frame_exit: m.u8(0x5099b0)?,
            unit_counts,
            structure_counts,
            unit_kind_counts,
            local_unit_count: m.u16(0x4e1a3a)?,
            other_unit_count: m.u16(0x4e1a38)?,
        })
    }
    fn project(&self, m: &mut MemoriaJogo) -> Result<()> {
        m.set_u32(0x496d40, self.game_frame)?;
        m.set_u8(0x496fe4, self.mission)?;
        m.set_u32(0x498db4, self.ai_rng_seed)?;
        m.set_u8(0x4e05a0, self.end_condition)?;
        m.set_u8(0x4e05a2, self.hud_refresh)?;
        m.set_u32(0x4e061c, self.end_condition_ticks)?;
        m.set_u8(0x4e0730, self.network)?;
        m.set_u8(0x4e1d18, self.permutation_cursor)?;
        m.set_u8(0x4e2480, self.input_player)?;
        m.set_u8(0x4e2f28, self.music_disabled)?;
        m.set_u8(0x4e2f78, self.game_state)?;
        m.set_u8(0x508041, self.local_player)?;
        m.set_u16(0x508720, self.view_x)?;
        m.set_u16(0x508722, self.view_y)?;
        m.set_u16(0x508724, self.view_width)?;
        m.set_u16(0x508726, self.view_height)?;
        m.set_u32(0x508f7c, self.iteration)?;
        m.set_u32(0x508f80, self.iteration_secondary)?;
        m.set_u32(0x508f84, self.iteration_tertiary)?;
        m.set_u16(0x5099a8, self.cursor_x)?;
        m.set_u16(0x5099aa, self.cursor_y)?;
        m.set_u8(0x5099b0, self.frame_exit)?;
        for (i, value) in self.unit_counts.iter().enumerate() {
            m.set_u16(0x4e1928 + 2 * i as u32, *value)?;
        }
        for (i, value) in self.structure_counts.iter().enumerate() {
            m.set_u32(0x4dfaa8 + 4 * i as u32, *value)?;
        }
        for (owner, kinds) in self.unit_kind_counts.iter().enumerate() {
            for (kind, value) in kinds.iter().enumerate() {
                m.set_u16(0x4e1a3c + owner as u32 * 0x34 + kind as u32 * 2, *value)?;
            }
        }
        m.set_u16(0x4e1a3a, self.local_unit_count)?;
        m.set_u16(0x4e1a38, self.other_unit_count)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameState {
    pub units: Vec<UnitState>,
    pub structures: Vec<StructureState>,
    pub globals: FrameGlobals,
    pub runtime: Option<crate::RuntimeState>,
    runtime_bound: bool,
    unit_pool_base: u32,
}

// Scope detection is deliberately structural: a sparse routine fixture is
// not a complete mission. Missing regions leave FrameState absent; no field
// value or missing part of a pool is synthesized.
fn declared(m: &MemoriaJogo, address: u32, size: usize) -> Result<bool> {
    match m.read(address, size) {
        Ok(_) => Ok(true),
        Err(EngineError::UndeclaredAccess { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

impl FrameState {
    pub fn unit_pool_base(&self) -> u32 {
        self.unit_pool_base
    }

    fn layout(m: &MemoriaJogo) -> Result<Option<u32>> {
        let Some(base) = m.optional_u32(UNIT_POOL_POINTER)? else {
            return Ok(None);
        };
        if !declared(m, base, UNIT_COUNT * STRIDE as usize)?
            || !declared(m, STRUCTURE_POOL, STRUCTURE_COUNT * STRIDE as usize)?
        {
            return Ok(None);
        }
        for (address, size) in [
            (0x496d40, 4),
            (0x496fe4, 1),
            (0x498db4, 4),
            (0x4e05a0, 1),
            (0x4e05a2, 1),
            (0x4e061c, 4),
            (0x4e0730, 1),
            (0x4e1d18, 1),
            (0x4e2480, 1),
            (0x4e2f28, 1),
            (0x4e2f78, 1),
            (0x508041, 1),
            (0x508720, 2),
            (0x508722, 2),
            (0x508724, 2),
            (0x508726, 2),
            (0x508f7c, 4),
            (0x508f80, 4),
            (0x508f84, 4),
            (0x5099a8, 2),
            (0x5099aa, 2),
            (0x5099b0, 1),
            (0x4e1928, 10),
            (0x4dfaa8, 16),
            (0x4e1a38, 4),
            (0x4e1a3c, 208),
        ] {
            if !declared(m, address, size)? {
                return Ok(None);
            }
        }
        Ok(Some(base))
    }

    pub fn load(m: &MemoriaJogo) -> Result<Option<Self>> {
        let Some(base) = Self::layout(m)? else {
            return Ok(None);
        };
        let units = (0..UNIT_COUNT)
            .map(|i| UnitState::load(m, base.wrapping_add(i as u32 * STRIDE)))
            .collect::<Result<Vec<_>>>()?;
        let structures = (0..STRUCTURE_COUNT)
            .map(|i| StructureState::load(m, STRUCTURE_POOL + i as u32 * STRIDE))
            .collect::<Result<Vec<_>>>()?;
        let runtime = crate::RuntimeState::load(m)?;
        let runtime_bound = runtime.is_some();
        Ok(Some(Self {
            runtime,
            runtime_bound,
            units,
            structures,
            globals: FrameGlobals::load(m)?,
            unit_pool_base: base,
        }))
    }

    pub fn project(&self, m: &mut MemoriaJogo) -> Result<()> {
        if self.runtime.is_some() != self.runtime_bound {
            return Err(EngineError::ProjectionFieldMismatch { address: 0x512320 });
        }
        if self.units.len() != UNIT_COUNT
            || self.structures.len() != STRUCTURE_COUNT
            || Self::layout(m)? != Some(self.unit_pool_base)
        {
            return Err(EngineError::ProjectionFieldMismatch {
                address: UNIT_POOL_POINTER,
            });
        }
        for (i, unit) in self.units.iter().enumerate() {
            unit.project(m, self.unit_pool_base.wrapping_add(i as u32 * STRIDE))?;
        }
        for (i, structure) in self.structures.iter().enumerate() {
            structure.project(m, STRUCTURE_POOL + i as u32 * STRIDE)?;
        }
        self.globals.project(m)?;
        if let Some(runtime) = &self.runtime {
            runtime.project(m)?;
        }
        Ok(())
    }
}

/// Reused semantic updates prepared before publishing a projected transaction.
#[derive(Clone, Debug)]
pub(crate) struct FrameDelta {
    units: Vec<(usize, UnitState)>,
    structures: Vec<(usize, StructureState)>,
    globals: Option<FrameGlobals>,
    runtime: Option<crate::RuntimeState>,
    runtime_changed: bool,
}
impl FrameDelta {
    pub(crate) fn new(frame: &FrameState) -> Self {
        Self {
            units: Vec::with_capacity(UNIT_COUNT),
            structures: Vec::with_capacity(STRUCTURE_COUNT),
            globals: None,
            runtime: frame.runtime.clone(),
            runtime_changed: false,
        }
    }
}
impl FrameState {
    pub(crate) fn prepare_delta(&self, m: &MemoriaJogo, delta: &mut FrameDelta) -> Result<()> {
        let mut units = [false; UNIT_COUNT];
        let mut structures = [false; STRUCTURE_COUNT];
        delta.units.clear();
        delta.structures.clear();
        delta.runtime_changed = false;
        for (address, length) in m.changes() {
            for (base, dirty) in [
                (self.unit_pool_base, units.as_mut_slice()),
                (STRUCTURE_POOL, structures.as_mut_slice()),
            ] {
                let begin = u64::from(address).max(u64::from(base));
                let end = (u64::from(address) + length as u64)
                    .min(u64::from(base) + dirty.len() as u64 * 276);
                if begin < end {
                    let first = ((begin - u64::from(base)) / 276) as usize;
                    let last = ((end - 1 - u64::from(base)) / 276) as usize;
                    dirty[first..=last].fill(true);
                }
            }
            // Runtime globals and pools are independent from the frame globals.
            for (base, length2) in [
                (0x512320u32, 9600u32),
                (0x5148a0, 116),
                (0x51a9e0, 3200),
                (0x51b660, 490),
                (0x50ae80, 4),
                (0x50b134, 72),
            ] {
                if u64::from(address) < u64::from(base) + u64::from(length2)
                    && u64::from(base) < u64::from(address) + length as u64
                {
                    delta.runtime_changed = true;
                }
            }
        }
        for (i, dirty) in units.into_iter().enumerate() {
            if dirty {
                delta.units.push((
                    i,
                    UnitState::load(m, self.unit_pool_base + i as u32 * STRIDE)?,
                ));
            }
        }
        for (i, dirty) in structures.into_iter().enumerate() {
            if dirty {
                delta.structures.push((
                    i,
                    StructureState::load(m, STRUCTURE_POOL + i as u32 * STRIDE)?,
                ));
            }
        }
        delta.globals = Some(FrameGlobals::load(m)?);
        if delta.runtime_changed {
            if let Some(runtime) = &mut delta.runtime {
                runtime.reload(m)?;
            }
        }
        Ok(())
    }
    pub(crate) fn apply_delta(&mut self, delta: &mut FrameDelta) {
        for (i, value) in delta.units.drain(..) {
            self.units[i] = value;
        }
        for (i, value) in delta.structures.drain(..) {
            self.structures[i] = value;
        }
        self.globals = delta.globals.take().expect("prepared globals");
        if delta.runtime_changed {
            std::mem::swap(&mut self.runtime, &mut delta.runtime);
        }
    }
}
