//! Intact SortilegeEffet473510 branches 1 PETRIFICAÇÃO and 2 ESPELHO.
//! Resolved unit IDs only; order acceptance belongs to SetEnginLanceSortilege.
use crate::{
    frame::stop, frame_spawn, spell_effects, spell_queries, FrameRegisters, MemoriaJogo, Result,
};

pub const EFFECT: u32 = 0x473510;

/// Void-effects ABI. Saved ECX/ESI/EDI/EBP and ESP are preserved here;
/// scratch EAX/EBX/EDX are deliberately outside this semantic API.
pub fn entry(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    let spell = r.edx as u8;
    if !matches!(spell, 1 | 2) {
        return stop("SortilegeEffet remaining spell domain", EFFECT);
    }
    let caster = r.eax as u16;
    let target = r.ebx as u16;
    if spell == 1 {
        speech(m, caster, 31, r.ecx)?;
        effect(m, target, 1)?;
        // Two separate EnginGetCase calls occur after the effect setter.
        let from = m.u16(frame_spawn::unit(m, u32::from(target))?.wrapping_add(0x7c))?;
        let to = m.u16(frame_spawn::unit(m, u32::from(target))?.wrapping_add(0x7c))?;
        projectile(m, 31, from, to)?;
        mana(m, caster, 1)?;
    } else {
        let t = frame_spawn::unit(m, u32::from(target))?;
        let owner = m.u8(t)?;
        let cell = m.u16(frame_spawn::unit(m, u32::from(target))?.wrapping_add(0x7c))?;
        let kind = m.u8(frame_spawn::unit(m, u32::from(target))?.wrapping_add(1))?;
        let cell = find_place(m, cell, target)?;
        speech(m, caster, 32, r.ecx)?;
        if cell != 65535 {
            let copy = create(m, cell, kind, owner)? as u16;
            place(m, copy)?;
            if matches!(kind, 11 | 12 | 15) {
                let child = create(m, cell, 9, owner)? as u16;
                effect(m, child, 2)?;
                lodge(m, copy, child)?;
                let hp = m.u16(frame_spawn::unit(m, u32::from(target))?.wrapping_add(0x9a))?;
                let child_record = frame_spawn::unit(m, u32::from(child))?;
                m.set_u16(child_record.wrapping_add(0x9a), hp)?;
                if !matches!(
                    m.u8(frame_spawn::unit(m, u32::from(child))?.wrapping_add(2))?,
                    0 | 4
                ) {
                    m.set_u8(frame_spawn::unit(m, u32::from(child))?.wrapping_add(2), 14)?;
                    frame_spawn::release(m, u32::from(child))?;
                }
            }
            effect(m, copy, 2)?;
            projectile(m, 32, cell, cell)?;
            mana(m, caster, 2)?;
            let hp = m.u16(frame_spawn::unit(m, u32::from(target))?.wrapping_add(0x9a))?;
            let copy = frame_spawn::unit(m, u32::from(copy))?;
            m.set_u16(copy.wrapping_add(0x9a), hp)?;
        }
    }
    Ok(FrameRegisters {
        esp: r.esp.wrapping_add(4),
        ..r
    })
}

fn speech(m: &mut MemoriaJogo, caster: u16, id: u32, ecx: u32) -> Result<()> {
    if m.u8(frame_spawn::unit(m, u32::from(caster))?)? == m.u8(0x508041)? {
        crate::player_orders::ask(m, u32::from(caster), id, 0, ecx)?;
    }
    Ok(())
}
fn effect(m: &mut MemoriaJogo, target: u16, spell: u32) -> Result<()> {
    spell_effects::set_effect_entry(
        m,
        FrameRegisters {
            eax: u32::from(target),
            edx: spell,
            ..Default::default()
        },
    )?;
    Ok(())
}
fn mana(m: &mut MemoriaJogo, caster: u16, spell: u32) -> Result<()> {
    let cost = spell_queries::entry(
        m,
        spell_queries::GET_COUT_MANA,
        FrameRegisters {
            eax: spell,
            ..Default::default()
        },
    )?
    .eax as u8;
    let p = frame_spawn::unit(m, u32::from(caster))?.wrapping_add(0xda);
    m.set_u8(p, m.u8(p)?.wrapping_sub(cost))
}

/// EnginCherchePlace4436ab: radii1..5, left/right then top/bottom,
/// fixed256 bounds (not mission dimensions). The target drives passability.
pub fn find_place(m: &mut MemoriaJogo, center: u16, target: u16) -> Result<u16> {
    let x = i32::from(center & 255);
    let y = i32::from(center >> 8);
    for k in 1..6i32 {
        let mut left = center.wrapping_sub((k * 257) as u16);
        let mut right = center.wrapping_add((k - k * 256) as u16);
        for i in -k..=k {
            let cells = [
                (x - k >= 0 && (0..256).contains(&(y + i)), left),
                (x + k < 256 && (0..256).contains(&(y + i)), right),
                (
                    i != -k && i != k && y - k >= 0 && (0..256).contains(&(x + i)),
                    center.wrapping_add((i - k * 256) as u16),
                ),
                (
                    i != -k && i != k && y + k < 256 && (0..256).contains(&(x + i)),
                    center.wrapping_add((i + k * 256) as u16),
                ),
            ];
            for (inside, cell) in cells {
                if inside && passable(m, target, cell)? {
                    return Ok(cell);
                }
            }
            left = (left & 255) | u16::from(((left >> 8) as u8).wrapping_add(1)) << 8;
            right = (right & 255) | u16::from(((right >> 8) as u8).wrapping_add(1)) << 8;
        }
    }
    Ok(65535)
}

// EnginPeutPasser442668, mode1 and null blocker-list as called by4436ab.
// Every occupied-cell query still runs; no invented collision answer is used.
fn passable(m: &mut MemoriaJogo, target: u16, cell: u16) -> Result<bool> {
    let u = frame_spawn::unit(m, u32::from(target))?;
    let kind = m.u8(u.wrapping_add(1))?;
    if kind > 25 {
        return Ok(true);
    } // Native direct442719 CL=1 branch.
    let quad = matches!(kind, 3 | 11..=16 | 25);
    if quad && (cell & 255 == 0 || cell >> 8 == 0) {
        return Ok(false);
    }
    let mut allowed = true;
    for &offset in if quad {
        &[0u16, 1, 257, 256][..]
    } else {
        &[0u16][..]
    } {
        let c = cell.wrapping_sub(offset);
        if crate::player_build::occupies_mode(m, u32::from(target), u32::from(c), 1)? {
            continue;
        }
        if m.u8(m.u32(0x507da8)?.wrapping_add(u32::from(c)))? & 1 == 0 {
            continue;
        }
        let blocker = crate::frame_movement::call(m, "ChercheIndiceEngin", &[u32::from(c)])? as u16;
        let crush = if matches!(kind, 12 | 15) && blocker != 65535 {
            let v = frame_spawn::unit(m, u32::from(blocker))?;
            m.u8(v)? != m.u8(frame_spawn::unit(m, u32::from(target))?)?
                && matches!(m.u8(v.wrapping_add(1))?,0..=2|4..=10|18|19|22)
        } else {
            false
        };
        if !crush {
            allowed = false;
        }
    }
    Ok(allowed)
}

// EnginLogeEngin447694. ESPELHO invokes this only for new11/12/15 groups.
fn lodge(m: &mut MemoriaJogo, group: u16, child: u16) -> Result<()> {
    lodge_unit(m, group, child).map(|_| ())
}
/// EnginLogeEngin 0x447694 with its AL result: 1 when the child was lodged.
pub(crate) fn lodge_unit(m: &mut MemoriaJogo, group: u16, child: u16) -> Result<bool> {
    let u = frame_spawn::unit(m, u32::from(group))?;
    let v = frame_spawn::unit(m, u32::from(child))?;
    let kind = m.u8(u.wrapping_add(1))?;
    if !matches!(kind, 11..=17) || m.u16(u.wrapping_add(0xc))? <= m.u16(u.wrapping_add(0xa))? {
        return Ok(false);
    }
    if m.u8(u)? != 200 && m.u8(u)? != m.u8(v)? {
        return Ok(false);
    }
    if m.u8(u)? == 200 {
        m.set_u8(u, m.u8(v)?)?;
        let group_record = frame_spawn::unit(m, u32::from(group))?;
        let cell = m.u16(group_record.wrapping_add(8))?;
        let owner = m.u8(group_record)?;
        if !matches!(owner, 4 | 200) {
            let radius = u16::from(m.u8(group_record.wrapping_add(0xbb))?)
                .wrapping_add(u16::from(m.u8(group_record.wrapping_add(0xf5))?));
            crate::player_sight::area(m, cell, u32::from(radius.wrapping_add(2)), owner, 0)?;
            crate::player_sight::area(m, cell, u32::from(radius), owner, 1)?;
        }
        frame_spawn::minimap(m, owner, cell, 4)?;
        if m.u8(u)? == m.u8(0x508041)? {
            crate::player_orders::ask(m, u32::from(group), 39, 0, 0)?;
        }
        let cap = frame_spawn::call(
            m,
            "EnginTypeGetCapacite",
            &[u32::from(m.u8(u.wrapping_add(1))?)],
        )?;
        let bonus = crate::frame_economy::bonus(
            m,
            u32::from(m.u8(u)?),
            u32::from(m.u8(u.wrapping_add(1))?),
            1,
            0,
        )?;
        m.set_u16(
            frame_spawn::unit(m, u32::from(group))?.wrapping_add(0xc),
            cap.wrapping_add(bonus) as u16,
        )?;
    }
    let count = u32::from(m.u16(u.wrapping_add(0xa))?);
    m.set_u16(
        u.wrapping_add(0x70),
        m.u16(u.wrapping_add(0x70))?.wrapping_add(1),
    )?;
    m.set_u8(u.wrapping_add(0x4c + count), 1)?;
    m.set_u16(u.wrapping_add(0x10 + 2 * count), child)?;
    m.set_u16(u.wrapping_add(0xa), (count as u16).wrapping_add(1))?;
    m.set_u8(v.wrapping_add(2), 14)?;
    Ok(true)
}

/// Native publication order for stationary effect projectiles31/32 only.
/// This does not advance, expire or render a projectile.
fn projectile(m: &mut MemoriaJogo, kind: u8, from: u16, to: u16) -> Result<()> {
    let mut slot = 0;
    while slot < 100 {
        let p = m.u32(0x5080f0)?.wrapping_add(slot * 56);
        if m.u8(p.wrapping_add(2))? == 0 {
            break;
        }
        slot += 1;
    }
    if slot == 100 {
        return Ok(());
    }
    let p = m.u32(0x5080f0)?.wrapping_add(slot * 56);
    m.set_u8(p, kind)?;
    m.set_u16(p.wrapping_add(0x14), from)?;
    m.set_u16(p.wrapping_add(0x16), to)?;
    m.set_u8(p.wrapping_add(0x23), 0)?;
    m.set_u16(p.wrapping_add(12), (from & 255).wrapping_mul(32))?;
    m.set_u8(p.wrapping_add(1), 1)?;
    m.set_u32(p.wrapping_add(0x24), 0)?;
    m.set_u16(p.wrapping_add(0x28), 0)?;
    m.set_u8(p.wrapping_add(2), 1)?;
    m.set_u8(p.wrapping_add(0x18), 0)?;
    m.set_u8(p.wrapping_add(3), 0)?;
    m.set_u16(p.wrapping_add(0x1a), 12)?;
    m.set_u16(p.wrapping_add(14), (from >> 8).wrapping_mul(32))?;
    m.set_u16(p.wrapping_add(0x10), 0)?;
    m.set_u16(p.wrapping_add(0x12), 0)?;
    m.set_u8(p.wrapping_add(0x1f), 0)?;
    m.set_u16(p.wrapping_add(0x20), 0)?;
    m.set_u8(p.wrapping_add(0x22), 0)?;
    m.set_u8(p.wrapping_add(0x19), 0)?;
    m.set_u16(p.wrapping_add(0x1c), 0)
}

fn create(m: &mut MemoriaJogo, cell: u16, kind: u8, owner: u8) -> Result<u32> {
    let mut i = 0;
    while i < 800 && m.u8(frame_spawn::unit(m, i)?.wrapping_add(2))? != 0 {
        i += 1;
    }
    if i == 800 {
        return Ok(65535);
    }
    let u = frame_spawn::unit(m, i)?;
    m.set_u8(u, if matches!(kind, 11..=15) { 200 } else { owner })?;
    set_type(m, i, cell, kind)?;
    m.set_u8(u.wrapping_add(0xdc), 0)?;
    m.set_u8(u.wrapping_add(2), 7)?;
    m.set_u16(u.wrapping_add(0x9a), 100)?;
    m.set_u8(u.wrapping_add(0x72), 0)?;
    m.set_u16(u.wrapping_add(0x74), 0)?;
    m.set_u16(u.wrapping_add(0xc0), 65535)?;
    for (o, v) in [(0xbe, 255), (0xbc, 255), (0xbd, 0), (0xaf, 0)] {
        m.set_u8(u.wrapping_add(o), v)?;
    }
    m.set_u32(u.wrapping_add(0x104), u32::MAX)?;
    m.set_u32(u.wrapping_add(0x10c), u32::MAX)?;
    for o in [0xf5, 0xf6, 0xf7, 0xf8] {
        m.set_u8(u.wrapping_add(o), 0)?;
    }
    m.set_u32(u.wrapping_add(0x110), 0)?;
    let v = m.u16(u.wrapping_add(0x76))?;
    m.set_u8(u.wrapping_add(0x94), 1)?;
    m.set_u16(u.wrapping_add(4), v)?;
    let mut j = 0;
    while j < u32::from(m.u16(u.wrapping_add(0xc))?) {
        m.set_u8(u.wrapping_add(0x4c + j), 0)?;
        m.set_u16(u.wrapping_add(0x10 + 2 * j), 0)?;
        j += 1;
    }
    frame_spawn::en_arret(m, i)?;
    let u = frame_spawn::unit(m, i)?;
    if m.u8(u.wrapping_add(1))? == 4 {
        let bonus = crate::frame_economy::bonus(m, u32::from(m.u8(u)?), 4, 12, 0)?;
        m.set_u8(
            frame_spawn::unit(m, i)?.wrapping_add(0xda),
            (bonus as u8).wrapping_add(10),
        )?;
    }
    let kind = m.u8(frame_spawn::unit(m, i)?.wrapping_add(1))?;
    if matches!(kind, 14 | 16) {
        m.set_u8(frame_spawn::unit(m, i)?.wrapping_add(0xda), 0)?;
    }
    Ok(i)
}
fn set_type(m: &mut MemoriaJogo, i: u32, cell: u16, kind: u8) -> Result<()> {
    let u = frame_spawn::unit(m, i)?;
    m.set_u16(u.wrapping_add(8), cell)?;
    m.set_u16(u.wrapping_add(0x7c), cell)?;
    let v = frame_spawn::unit(m, i)?;
    m.set_u16(v.wrapping_add(0x7e), cell)?;
    frame_spawn::release(m, i)?;
    m.set_u8(v.wrapping_add(0x88), 0)?;
    m.set_u16(u.wrapping_add(0xb2), 65535)?;
    for o in [0xab, 0x97, 0x95, 0x96, 0x92, 0x86, 0x87, 0xb6, 0xaa] {
        m.set_u8(u.wrapping_add(o), 0)?;
    }
    m.set_u8(u.wrapping_add(0x9c), 99)?;
    for o in [0xa4, 0xa9, 0x9e] {
        m.set_u8(u.wrapping_add(o), 0)?;
    }
    m.set_u16(u.wrapping_add(0xb8), 65535)?;
    m.set_u16(u.wrapping_add(0x8c), 0)?;
    m.set_u8(u.wrapping_add(0x8e), 0)?;
    for o in [0x90, 0x91, 0x94] {
        m.set_u8(u.wrapping_add(o), 1)?;
    }
    m.set_u8(u.wrapping_add(1), kind)?;
    for o in [0x88, 0x8b, 0xbc, 0xbd] {
        m.set_u8(u.wrapping_add(o), 0)?;
    }
    let v = frame_spawn::unit(m, i)?;
    if !matches!(m.u8(v.wrapping_add(2))?, 0 | 4) {
        m.set_u8(v.wrapping_add(2), 7)?;
        frame_spawn::release(m, i)?;
    }
    m.set_u16(u.wrapping_add(4), 4)?;
    m.set_u16(u.wrapping_add(0x74), 0)?;
    let sight = m.u8(0x4965d8 + 50 * u32::from(kind))?;
    m.set_u8(u.wrapping_add(0xbb), sight)?;
    let speed = m.u8(0x4965da + 50 * u32::from(kind))?;
    m.set_u16(u.wrapping_add(0x76), 4)?;
    m.set_u8(u.wrapping_add(6), 4)?;
    m.set_u16(u.wrapping_add(0xc2), 0)?;
    m.set_u8(u.wrapping_add(0x78), 0)?;
    m.set_u8(u.wrapping_add(0xb0), speed)?;
    frame_spawn::call(m, "EnginSetTypeCaract", &[i])?;
    let cap = frame_spawn::call(m, "EnginTypeGetCapacite", &[u32::from(kind)])?;
    let bonus = crate::frame_economy::bonus(
        m,
        u32::from(m.u8(u)?),
        u32::from(m.u8(u.wrapping_add(1))?),
        1,
        0,
    )?;
    let u = frame_spawn::unit(m, i)?;
    for o in [0x70, 0xe, 0xc, 0xa] {
        m.set_u16(u.wrapping_add(o), 0)?;
    }
    for j in 0..30 {
        m.set_u16(u.wrapping_add(0x10 + 2 * j), 0)?;
        m.set_u8(u.wrapping_add(0x4c + j), 0)?;
    }
    m.set_u16(u.wrapping_add(0xc), cap.wrapping_add(bonus) as u16)
}
fn place(m: &mut MemoriaJogo, index: u16) -> Result<()> {
    let u = frame_spawn::unit(m, u32::from(index))?;
    crate::frame_core::set_unit_collision(
        m,
        u32::from(index),
        u32::from(m.u16(u.wrapping_add(8))?),
    )?;
    m.set_u8(u.wrapping_add(0x72), 1)?;
    let u = frame_spawn::unit(m, u32::from(index))?;
    let owner = m.u8(u)?;
    let cell = m.u16(u.wrapping_add(8))?;
    if !matches!(owner, 4 | 200) {
        let radius = u16::from(m.u8(u.wrapping_add(0xbb))?)
            .wrapping_add(u16::from(m.u8(u.wrapping_add(0xf5))?));
        crate::player_sight::area(m, cell, u32::from(radius.wrapping_add(2)), owner, 0)?;
        crate::player_sight::area(m, cell, u32::from(radius), owner, 1)?;
    }
    frame_spawn::minimap(m, owner, cell, 4)?;
    Ok(())
}

/// Exact4529fe argument transport, effect call only; no ResetOrdre claim.
pub fn attack_call(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    let mut out = entry(m, spell_effects::attack_arguments(m, r)?)?;
    out.esp = r.esp;
    Ok(out)
}
