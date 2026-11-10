//! Bounded VISION / NOVA SortilegeEffet bodies, directly from intact x86.
//! This API begins at the resolved effect call; it accepts no player order.
use crate::{frame::stop, spell_queries, FrameRegisters, MemoriaJogo, Result};

pub const EFFECT: u32 = 0x473510;

/// AX=caster, BX=target cell, DL=0 (VISION) or 4 (NOVA).
/// Target is a cell, never a unit index. Failures retain native write prefixes.
pub fn entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let spell = r.edx as u8;
    if !matches!(spell, 0 | 4) {
        return stop(
            "SortilegeEffet requires independently ported spell body",
            EFFECT,
        );
    }
    let caster = u32::from(r.eax as u16);
    let target = r.ebx as u16;
    let unit = crate::player::unit(m, caster)?;
    if m.u8(unit)? == m.u8(0x508041)? {
        crate::player_orders::ask(m, caster, if spell == 0 { 30 } else { 34 }, 0, r.ecx)?;
    }
    // NOVA calls EnginGetCase (+7c) after speech; Ask uses the distinct +8 cell.
    let from = if spell == 0 {
        target
    } else {
        m.u16(crate::player::unit(m, caster)?.wrapping_add(0x7c))?
    };
    let scratch = projectile(m, spell, from, target, caster)?;
    r.ebx = scratch;
    let mana = spell_queries::entry(
        m,
        spell_queries::GET_COUT_MANA,
        FrameRegisters {
            eax: u32::from(spell),
            ..r
        },
    )?
    .eax as u8;
    let pool = m.u32(0x4e19a0)?;
    let offset = caster.wrapping_mul(276);
    let address = pool.wrapping_add(offset).wrapping_add(0xda);
    m.set_u8(address, m.u8(address)?.wrapping_sub(mana))?;
    r.eax = pool;
    r.edx = offset;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Selected ProjectileInit/Set publication. Reuses the proved line primitive;
/// the general semantic Set provider groups its stores differently from x86.
fn projectile(m: &mut MemoriaJogo, spell: u8, from: u16, to: u16, caster: u32) -> Result<u32> {
    let mut slot = 0;
    // Native re-reads the pool pointer at each probe and again before Set.
    while slot < 100 {
        let p = m.u32(0x5080f0)?.wrapping_add(slot * 56);
        if m.u8(p.wrapping_add(2))? == 0 {
            break;
        }
        slot += 1;
    }
    if slot == 100 {
        return Ok(99 * 56);
    }
    let p = m.u32(0x5080f0)?.wrapping_add(slot * 56);
    m.set_u8(p, if spell == 0 { 28 } else { 29 })?;
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
    m.set_u8(p.wrapping_add(0x1f), u8::from(spell == 4))?;
    m.set_u16(
        p.wrapping_add(0x20),
        if spell == 0 { 0 } else { caster as u16 },
    )?;
    m.set_u8(p.wrapping_add(0x22), 0)?;
    m.set_u8(p.wrapping_add(0x19), 0)?;
    m.set_u16(p.wrapping_add(0x1c), 0)?;
    if spell == 4 {
        m.set_u8(p.wrapping_add(0x19), 0)?;
        crate::frame_combat::call(m, "ProjectileLineaire", &[slot])?;
        m.set_u16(p.wrapping_add(0x1a), 20)?;
        return Ok(p);
    }
    let radius = spell_queries::entry(m, spell_queries::GET_RAYON_EFFET, FrameRegisters::default())?
        .eax as u16;
    crate::player_sight::area(m, to, u32::from(radius.wrapping_add(2)), 0, 0)?;
    // The original repeats the table lookup after discovery, including aliases.
    let radius = spell_queries::entry(m, spell_queries::GET_RAYON_EFFET, FrameRegisters::default())?
        .eax as u16;
    crate::player_sight::area(m, to, u32::from(radius), 0, 1)?;
    // The final outer-loop increment loads the signed Y counter into EBX,
    // then increments it, even when every row is clipped (476ddf..476de6).
    Ok(u32::from(radius) + 1)
}

/// Caller slice through452a19, before ResetOrdre. Caller must reach state24.
pub fn attack_call(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    let mut out = entry(m, crate::spell_effects::attack_arguments(m, r)?)?;
    out.esp = r.esp;
    Ok(out)
}
