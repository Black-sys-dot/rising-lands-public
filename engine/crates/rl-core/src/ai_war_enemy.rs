//! Intact iagWarAddEnnemyToBattle 0x487b78 and iagWarAffecteEnnemy 0x487eef.
//! Effects/AL API: full EAX player, EDX battle, EBX encoded reference.
//! Upper EBX participates in duplicate comparison; only low16 is stored.
use crate::{
    ai_group_message::CityMessage, ai_war_position::WarEnemy, EngineError, MemoriaJogo, Result,
};

fn unit(m: &MemoriaJogo, index: u32) -> Result<u32> {
    Ok(m.u32(0x4e19a0)?.wrapping_add(index.wrapping_mul(276)))
}
fn reference(m: &MemoriaJogo, encoded: u16) -> Result<u32> {
    if encoded & 0x8000 != 0 {
        Ok(0x49a0c8u32.wrapping_add(u32::from(encoded & 0x7fff).wrapping_mul(276)))
    } else {
        unit(m, u32::from(encoded))
    }
}
fn health(m: &MemoriaJogo, encoded: u16) -> Result<i32> {
    let r = reference(m, encoded)?;
    if encoded & 0x8000 != 0 {
        Ok(
            i32::from(m.u16(r.wrapping_add(10))?)
                .wrapping_sub(i32::from(m.u16(r.wrapping_add(6))?)),
        )
    } else {
        Ok(i32::from(m.u16(r.wrapping_add(0x9a))?))
    }
}
fn entry(battle: u32, index: u32) -> u32 {
    battle.wrapping_add(index.wrapping_mul(8)).wrapping_add(16)
}

/// Full-width addressing, owner equality before the capacity test, no alliance gate.
/// Append writes priority/reference/zero; insertion copies two dwords at a time
/// backwards then writes zero/priority/reference. Unused row bytes remain intact.
pub fn add(m: &mut MemoriaJogo, player: u32, battle: u32, encoded: u32) -> Result<u8> {
    let low = encoded as u16;
    let r = reference(m, low)?;
    let owner = u32::from(m.u8(r.wrapping_add(if low & 0x8000 != 0 { 4 } else { 0 }))?);
    if owner == player {
        return Ok(0);
    }
    let b = 0x51bd24u32
        .wrapping_add(player.wrapping_mul(0x1144))
        .wrapping_add(battle.wrapping_mul(0x110));
    if m.u16(b.wrapping_add(14))? >= 32 {
        return Ok(0);
    }
    let building = low & 0x8000 != 0;
    let kind = u32::from(m.u8(r.wrapping_add(if building { 8 } else { 1 }))?);
    let priority = u16::from(m.u32(
        (if building { 0x498d5cu32 } else { 0x498cf4u32 }).wrapping_add(kind.wrapping_mul(4)),
    )? as u8);
    let mut candidate = None;
    let mut insertion = None;
    let mut index = 0;
    while index < u32::from(m.u16(b.wrapping_add(14))?) && insertion.is_none() {
        let e = entry(b, index);
        let old = m.u16(e)?;
        if u32::from(old) == encoded {
            if m.u16(e.wrapping_add(2))? == 0 {
                assign(m, b, index)?;
            }
            return Ok(0);
        }
        let old_priority = m.u16(e.wrapping_add(4))?;
        if old_priority == priority {
            // The incoming health read precedes the old reference health read.
            let incoming = health(m, low)?;
            if incoming < health(m, old)? {
                candidate = Some(index);
            }
        } else if old_priority < priority {
            insertion = Some(candidate.unwrap_or(index));
        }
        index = index.wrapping_add(1);
    }
    let position;
    if let Some(at) = insertion {
        let count = u32::from(m.u16(b.wrapping_add(14))?);
        for i in (at..count).rev() {
            let e = entry(b, i);
            m.set_u32(e.wrapping_add(8), m.u32(e)?)?;
            m.set_u32(e.wrapping_add(12), m.u32(e.wrapping_add(4))?)?;
        }
        let e = entry(b, at);
        m.set_u16(e.wrapping_add(2), 0)?;
        m.set_u16(e.wrapping_add(4), priority)?;
        m.set_u16(e, low)?;
        position = at;
    } else {
        let at = u32::from(m.u16(b.wrapping_add(14))?);
        m.set_u16(entry(b, at).wrapping_add(4), priority)?;
        let at = u32::from(m.u16(b.wrapping_add(14))?);
        m.set_u16(entry(b, at), low)?;
        let at = u32::from(m.u16(b.wrapping_add(14))?);
        m.set_u16(entry(b, at).wrapping_add(2), 0)?;
        position = u32::from(m.u16(b.wrapping_add(14))?);
    }
    let count = m.u16(b.wrapping_add(14))?;
    m.set_u16(b.wrapping_add(14), count.wrapping_add(1))?;
    assign(m, b, position)?;
    Ok(1)
}

/// Original near pass, then unrestricted pass. Group arithmetic wraps before
/// the null-address test; group index ffffffff is not an invented null group.
/// Reads the head before the count, and only stores assigned damage at the end.
pub fn assign(m: &mut MemoriaJogo, battle: u32, index: u32) -> Result<()> {
    let g = 0x51a9e0u32.wrapping_add(m.u32(battle)?.wrapping_shl(5));
    if g == 0 {
        return Ok(());
    }
    let head = m.u16(g.wrapping_add(16))?;
    if m.u16(g.wrapping_add(18))? == 0 || head == 0xffff {
        return Ok(());
    }
    let e = entry(battle, index);
    let mut assigned = u32::from(m.u16(e.wrapping_add(2))?);
    let target = m.u16(e)?;
    let hp = health(m, target)?;
    let mut traversals = 0;
    for near in [true, false] {
        if (assigned as i32) >= hp {
            break;
        }
        let mut member = m.u16(g.wrapping_add(16))?;
        while member != 0xffff && (assigned as i32) < hp {
            if traversals == 10000 {
                return Err(EngineError::UnportedCallee {
                    name: "iagWarAffecteEnnemy bounded list traversal",
                    address: 0x488003,
                });
            }
            traversals += 1;
            let r = unit(m, u32::from(member))?;
            if m.u16(r.wrapping_add(0xb8))? == 0xffff && (!near || in_range(m, member, target)?) {
                let source = u32::from(m.u8(r.wrapping_add(1))?);
                let target_kind = if target & 0x8000 != 0 {
                    26
                } else {
                    u32::from(m.u8(unit(m, u32::from(target))?.wrapping_add(1))?)
                };
                let table = m.u32(m.u32(0x4e199c)?.wrapping_add(source.wrapping_mul(4)))?;
                let damage = u32::from(m.u8(table.wrapping_add(target_kind))?);
                if damage > 0 {
                    assigned = assigned.wrapping_add(damage);
                    set_attack(m, member, target)?;
                }
            }
            member = m.u16(r.wrapping_add(0x108))?;
        }
    }
    m.set_u16(e.wrapping_add(2), assigned as u16)
}

/// EnginAPortee uses the existing intact bounded range primitives. Their
/// addressed spell/size/habitability dependencies propagate unchanged.
fn in_range(m: &mut MemoriaJogo, member: u16, target: u16) -> Result<bool> {
    crate::chain_battle::in_range(m, member, target)
}

/// SetAttaqueEngin 445a36: exact mode1 vs mode3 write order after ResetOrdre.
/// ResetOrdre below preserves its own release-store order.
pub fn set_attack(m: &mut MemoriaJogo, member: u16, target: u16) -> Result<()> {
    let r = unit(m, u32::from(member))?;
    let mode = if target & 0x8000 != 0 { 3 } else { 1 };
    let target = target & 0x7fff;
    if m.u8(r.wrapping_add(2))? == 1
        && m.u8(r.wrapping_add(0xb7))? == mode
        && m.u16(r.wrapping_add(0xb8))? == target
    {
        return Ok(());
    }
    reset_order(m, member)?;
    if matches!(m.u8(r.wrapping_add(1))?, 5 | 6 | 7 | 9 | 10 | 13) {
        return Ok(());
    }
    if mode == 1 {
        if member == target {
            return Ok(());
        }
        m.set_u8(r.wrapping_add(0x8a), 0)?;
        m.set_u8(r.wrapping_add(2), mode)?;
    } else {
        m.set_u8(r.wrapping_add(2), 1)?;
        m.set_u8(r.wrapping_add(0x8a), 0)?;
    }
    m.set_u8(r.wrapping_add(0xb7), mode)?;
    m.set_u16(r.wrapping_add(0xb8), target)
}

/// ResetOrdre44357c and its three tiny construction leaves. The reservation
/// owner word clears before the pointer dword (44361b before443622).
fn reset_order(m: &mut MemoriaJogo, member: u16) -> Result<()> {
    let r = unit(m, u32::from(member))?;
    if m.u8(r.wrapping_add(0x86))? == 0
        && m.u8(r.wrapping_add(0x87))? == 0
        && m.u8(r.wrapping_add(1))? != 16
    {
        m.set_u8(r.wrapping_add(0xa9), 0)?;
        m.set_u8(r.wrapping_add(0xa4), 0)?;
    }
    if m.u8(r.wrapping_add(1))? == 13 {
        let structure = m.u16(r.wrapping_add(0xc0))?;
        if structure != 0xffff {
            let b = 0x49a0c8u32.wrapping_add(u32::from(structure).wrapping_mul(276));
            m.set_u8(b.wrapping_add(0x74), 0)?;
            if m.u8(b.wrapping_add(0x76))? == 15 && m.u8(b.wrapping_add(0x75))? != 0 {
                m.set_u8(b.wrapping_add(0x75), 0)?;
            }
        }
    }
    m.set_u16(r.wrapping_add(0x7e), m.u16(r.wrapping_add(0x7c))?)?;
    if m.u16(0x496304)? == member {
        m.set_u8(0x4e0f36, 0)?;
        m.set_u8(0x4e0f37, 0)?;
        m.set_u16(0x496304, 0xffff)?;
        m.set_u32(0x4e0f3c, 0)?;
    }
    m.set_u8(r.wrapping_add(0x88), 0)?;
    m.set_u8(r.wrapping_add(0x82), 0)?;
    m.set_u16(r.wrapping_add(0xb2), 0xffff)?;
    m.set_u8(r.wrapping_add(2), 2)
}

pub fn consume(m: &mut MemoriaJogo, call: WarEnemy) -> Result<()> {
    add(m, call.player, call.battle, u32::from(call.unit)).map(|_| ())
}
pub fn send(m: &mut MemoriaJogo, structure: u16, message: u8, argument: u32) -> Result<()> {
    crate::ai_war_position::send_with_enemy(m, structure, message, argument, consume)
}
pub fn descriptor(m: &mut MemoriaJogo, message: CityMessage) -> Result<()> {
    send(m, message.structure, message.message, 0)
}
