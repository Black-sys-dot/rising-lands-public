//! Bounded intact iagVilleSendMessage 0x484808 and city wait-list 0x48499d.
use crate::{ai_group_message::CityMessage, EngineError, MemoriaJogo, Result};

fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}
fn task(index: u32) -> u32 {
    0x512320u32.wrapping_add(index.wrapping_mul(96))
}

// Intact 0x486025/0x486065: first free/matching resource-building slot only.
fn resource_slot(m: &mut MemoriaJogo, owner: u32, structure: u16, add: bool) -> Result<()> {
    let base = 0x51ba3cu32.wrapping_add(owner.wrapping_mul(168));
    for index in 0..8 {
        let address = base.wrapping_add(0x8c + 2 * index);
        if m.u16(address)? == if add { u16::MAX } else { structure } {
            m.set_u16(address, if add { structure } else { u16::MAX })?;
            break;
        }
    }
    Ok(())
}

/// Original 0x48499d..0x484a7d. The predecessor advances even when the
/// current node was removed; retain that observable native list-store order.
fn notify(m: &mut MemoriaJogo, city: u32, message: u8, value: u32) -> Result<()> {
    let mask = 1u32.wrapping_shl(u32::from(message));
    let mut previous = u32::MAX;
    let mut current = m.u32(city.wrapping_add(0x68))?;
    for _ in 0..10000 {
        if current == u32::MAX {
            return Ok(());
        }
        let t = task(current);
        let next = m.u32(t.wrapping_add(8))?;
        let flags = u32::from(m.u8(t.wrapping_add(0x5d))?);
        let matches = m.u32(t.wrapping_add(0x10))? == value;
        let wake = matches && (message != 1 || flags == mask);
        if wake {
            crate::frame_tasks::wake(m, current, if flags & mask != 0 { 1 } else { u32::MAX })?;
            if current == m.u32(city.wrapping_add(0x68))? {
                m.set_u32(city.wrapping_add(0x68), m.u32(t.wrapping_add(8))?)?;
            } else {
                m.set_u32(task(previous).wrapping_add(8), m.u32(t.wrapping_add(8))?)?;
            }
            m.set_u8(t.wrapping_add(0x5d), 0xff)?;
        }
        previous = current;
        current = next;
    }
    Err(stop("iagVilleSendMessage bounded wait list", 0x4849c5))
}

/// AX=structure, DL=message, EBX=argument; effects only, no scratch GPR return.
/// ECX/ESI/EDI/EBP survive; RET consumes four bytes. Owner and PlayerGetType
/// gates precede message dispatch. All native prefix writes survive an error.
/// Message4 stops at its real war query, after the real unit-position read.
pub fn send(m: &mut MemoriaJogo, structure: u16, message: u8, argument: u32) -> Result<()> {
    let record = 0x49a0c8u32.wrapping_add(276 * u32::from(structure));
    let owner = u32::from(m.u8(record.wrapping_add(4))?);
    if owner >= 4 {
        return Ok(());
    }
    if crate::frame_economy::call(m, "PlayerGetType", &[owner])? & 255 != 2 {
        return Ok(());
    }
    let city = 0x51b84cu32.wrapping_add(124 * owner);
    match message {
        1 => {
            let kind = m.u8(record.wrapping_add(8))?;
            if kind == 3 {
                resource_slot(m, owner, structure, true)?;
            }
            notify(m, city, message, u32::from(kind))
        }
        2 => {
            let kind = u32::from(m.u8(record.wrapping_add(8))?);
            let count = city.wrapping_add(2 * kind);
            m.set_u16(count, m.u16(count)?.wrapping_sub(1))?;
            m.set_u32(
                city.wrapping_add(0x6c),
                m.u32(city.wrapping_add(0x6c))?.wrapping_sub(1),
            )?;
            if kind == 3 {
                resource_slot(m, owner, structure, false)?;
            }
            notify(m, city, message, u32::from(structure))
        }
        3 => notify(m, city, message, u32::from(structure)),
        4 => {
            let unit = argument as u16;
            let unit_record = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(unit));
            let _position = m.u16(unit_record.wrapping_add(0x7c))?;
            Err(stop("iagVilleSendMessage war query", 0x487594))
        }
        _ => Ok(()),
    }
}

/// Actual group-descriptor callback contract: EBX is the native literal zero;
/// target's full ECX survives the city body and is not an alternate argument.
pub fn descriptor(m: &mut MemoriaJogo, message: CityMessage) -> Result<()> {
    send(m, message.structure, message.message, 0)
}

/// iagVilleAttendMessage 0x48491f through I_AttenteBatimentConstruit 0x482679
/// (message 2, arguments [structure]) and I_AttenteMessageBatiment 0x482697
/// (arguments [structure, message]); player at +0x1c, task at +0x20. The task
/// is appended to the player's city wait list, records the structure and
/// message byte, then sleeps with time 0. Returns 1 unconditionally.
pub fn wait_building(
    m: &mut MemoriaJogo,
    address: u32,
    arguments: &[u32],
    player: u32,
    task_index: u32,
) -> Result<u32> {
    let (structure, message) = match (address, arguments) {
        (0x482679, [structure, ..]) => (*structure, 2),
        (0x482697, [structure, message, ..]) => (*structure, *message),
        _ => return Err(stop("iagVilleAttendMessage missing arguments", address)),
    };
    let head = 0x51b8b4u32.wrapping_add(player.wrapping_mul(124));
    let mut node = m.u32(head)?;
    if node == u32::MAX {
        m.set_u32(head, task_index)?;
    } else {
        for _ in 0..10000 {
            let next = m.u32(task(node).wrapping_add(8))?;
            if next == u32::MAX {
                break;
            }
            node = next;
        }
        if m.u32(task(node).wrapping_add(8))? != u32::MAX {
            return Err(stop("iagVilleAttendMessage bounded wait list", 0x484959));
        }
        m.set_u32(task(node).wrapping_add(8), task_index)?;
    }
    let t = task(task_index);
    m.set_u32(t.wrapping_add(8), u32::MAX)?;
    m.set_u32(t.wrapping_add(0x10), structure)?;
    m.set_u8(t.wrapping_add(0x5d), message as u8)?;
    crate::frame_tasks::sleep(m, task_index, 0)?;
    Ok(1)
}

pub fn wait_building_entry(
    m: &mut MemoriaJogo,
    address: u32,
    mut r: crate::FrameRegisters,
) -> Result<crate::FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    let player = m.u32(r.eax.wrapping_add(0x1c))?;
    let task_index = m.u32(r.eax.wrapping_add(0x20))?;
    r.eax = wait_building(m, address, &arguments, player, task_index)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// iagRessRequest 0x485a54 through V_AttendreRessource 0x485d53 (arguments
/// [resource, amount]; player at +0x1c, task at +0x20). Only when the amount
/// is positive (signed): the player's pending counter at 0x51ba5c + 168 *
/// player + 4 * resource grows by the amount, the task is appended to the
/// player's resource wait list (head 0x51ba80 + 168 * player, links at task + 8),
/// records the amount (+0x10) and the resource byte (+0x5d), then sleeps with
/// time 0. Returns 1 unconditionally.
pub fn wait_resource(
    m: &mut MemoriaJogo,
    arguments: &[u32],
    player: u32,
    task_index: u32,
) -> Result<u32> {
    let [resource, amount, ..] = arguments else {
        return Err(stop("iagRessRequest missing arguments", 0x485d53));
    };
    if (*amount as i32) <= 0 {
        return Ok(1);
    }
    let offset = player.wrapping_mul(168);
    let counter = 0x51ba5cu32
        .wrapping_add(offset)
        .wrapping_add(resource.wrapping_mul(4));
    m.set_u32(counter, m.u32(counter)?.wrapping_add(*amount))?;
    let head = 0x51ba80u32.wrapping_add(offset);
    let mut node = m.u32(head)?;
    if node == u32::MAX {
        m.set_u32(head, task_index)?;
    } else {
        for _ in 0..10000 {
            let next = m.u32(task(node).wrapping_add(8))?;
            if next == u32::MAX {
                break;
            }
            node = next;
        }
        if m.u32(task(node).wrapping_add(8))? != u32::MAX {
            return Err(stop("iagRessRequest bounded wait list", 0x485aa0));
        }
        m.set_u32(task(node).wrapping_add(8), task_index)?;
    }
    let t = task(task_index);
    m.set_u32(t.wrapping_add(8), u32::MAX)?;
    m.set_u8(t.wrapping_add(0x5d), *resource as u8)?;
    m.set_u32(t.wrapping_add(0x10), *amount)?;
    crate::frame_tasks::sleep(m, task_index, 0)?;
    Ok(1)
}

pub fn wait_resource_entry(
    m: &mut MemoriaJogo,
    mut r: crate::FrameRegisters,
) -> Result<crate::FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    let player = m.u32(r.eax.wrapping_add(0x1c))?;
    let task_index = m.u32(r.eax.wrapping_add(0x20))?;
    r.eax = wait_resource(m, &arguments, player, task_index)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
