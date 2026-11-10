//! Original ARMADURA (spell 3), not a general spell dispatcher.
//! SortilegeEffet 0x473510 -> 0x47370e, and its required duration setter.
use crate::{frame::stop, spell_queries, FrameRegisters, MemoriaJogo, Result};

pub const SET_EFFECT: u32 = 0x44ff7f;
pub const ARMOR_EFFECT: u32 = 0x473510;

/// Already-resolved unit IDs at the effect call, not a player command.
/// Invention, range and order acceptance belong to the original callers.
#[derive(Debug, Clone, Copy)]
pub struct ArmorCast {
    pub caster: u16,
    pub target: u16,
}

/// Exact argument preparation at CalculAttaqueEngin 0x4529fe..0x452a14.
/// ESI is the caller's unit record, DI its unit ID. This only transports the
/// original spell byte and target word; it accepts no order and dispatches none.
pub fn attack_arguments(m: &MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    r.ebp = r.edi & 65535;
    r.ebx = u32::from(m.u16(r.esi.wrapping_add(0xb8))?);
    r.edx = u32::from(m.u8(r.esi.wrapping_add(0xf4))?);
    r.eax = r.ebp;
    Ok(r)
}

/// Selected native call through 0x452a19, before the separate ResetOrdre.
/// The caller must have reached its state24 firing branch. No animation,
/// targeting, order validation or post-call order reset is claimed here.
pub fn armor_attack_call(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    let mut returned = armor_entry(m, attack_arguments(m, r)?)?;
    // The caller's CALL pushes a return address; its callee RET consumes it.
    returned.esp = r.esp;
    Ok(returned)
}

/// Complete EnginSetSortilegeEffet register ABI. AX is a unit index, DL a bit
/// and duration-table index. Raw indices require explicitly declared memory;
/// only the five shipped indices are gameplay spells. Stores are not atomic.
pub fn set_effect_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let target = crate::player::unit(m, r.eax)?;
    let spell = r.edx & 255;
    crate::frame_core::call(m, "SetBit", &[target.wrapping_add(0xdc), spell, 1])?;
    let duration = spell_queries::entry(
        m,
        spell_queries::GET_TEMPS_FIN,
        FrameRegisters { eax: spell, ..r },
    )?
    .eax as u16;
    m.set_u32(
        target.wrapping_add(0xe0).wrapping_add(spell * 4),
        u32::from(duration),
    )?;
    r.eax = spell;
    r.edx = u32::from(duration);
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Entire selected SortilegeEffet path, including native scratch GPR results.
/// AX=caster, BX=target, DL=3; upper argument bits follow the original ABI.
/// Other spell bodies remain addressed errors before any mutation.
pub fn armor_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    if r.edx as u8 != 3 {
        return stop(
            "SortilegeEffet requires independently ported spell body",
            ARMOR_EFFECT,
        );
    }
    let cast = ArmorCast {
        caster: r.eax as u16,
        target: r.ebx as u16,
    };
    let caster = crate::player::unit(m, u32::from(cast.caster))?;
    // EnginGetPlayer, then local-player speech, before target/duration writes.
    if m.u8(caster)? == m.u8(0x508041)? {
        let player = u32::from(m.u8(0x508041)?);
        let cell = u32::from(m.u16(caster.wrapping_add(8))?);
        let discovered = m.u8(m.u32(0x507d78 + 4 * player)?.wrapping_add(cell))? == 0;
        let request = discovered && m.u32(caster.wrapping_add(0x104))? == u32::MAX;
        // Proven EnginAsk and its real unopened-sound provider; no callback
        // answer or display-boundary bypass is supplied by this API.
        crate::player_orders::ask(m, u32::from(cast.caster), 0x21, 0, r.ecx)?;
        // Native EBX: Ask's unit pointer, free handle when pool is full, or
        // HndSound_LoadDataInfo's empty bank pointer (48b390) on real failure.
        r.ebx = if request {
            if m.u32(0x498f68)? == 70 {
                caster.wrapping_add(0x104)
            } else {
                0x522000
            }
        } else {
            caster
        };
    }
    set_effect_entry(
        m,
        FrameRegisters {
            eax: u32::from(cast.target),
            edx: 3,
            ..r
        },
    )?;
    let mana = spell_queries::entry(
        m,
        spell_queries::GET_COUT_MANA,
        FrameRegisters { eax: 3, ..r },
    )?
    .eax as u8;
    // EnginRetireMana re-reads the pool after all target writes. No test for
    // sufficient mana occurs here; SUB byte wraps, including self-casts.
    let pool = m.u32(0x4e19a0)?;
    let offset = u32::from(cast.caster) * 276;
    let address = pool.wrapping_add(offset).wrapping_add(0xda);
    m.set_u8(address, m.u8(address)?.wrapping_sub(mana))?;
    r.eax = pool;
    r.edx = offset;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

pub fn armor(m: &mut MemoriaJogo, cast: ArmorCast) -> Result<()> {
    armor_entry(
        m,
        FrameRegisters {
            eax: u32::from(cast.caster),
            ebx: u32::from(cast.target),
            edx: 3,
            ..FrameRegisters::default()
        },
    )
    .map(|_| ())
}
