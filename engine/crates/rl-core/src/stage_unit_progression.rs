//! EnginNextEtape's native loaded type dispatch and RotateTourEngin4484a7.
//! Narrow adoption helpers; no type16/17, animation, sound or renderer admission.
use crate::{frame::stop, player::unit, MemoriaJogo, Result};

/// Native4545af..4545f3. The caller supplies its original cached kind/record.
pub fn after_reload(m: &MemoriaJogo, r: u32, kind: u8) -> Result<()> {
    if matches!(kind, 20..=24) {
        //45459a rejects only16/17;4545af subtracts11 and falls through
        //4545c6 with false for these five actually loaded creature types.
        return Ok(());
    }
    if !(11..=15).contains(&kind) {
        return stop("stage progression type lane", 0x4545b9);
    }
    let target = m.u32(0x45432c + 4 * u32::from(kind - 11))?;
    if target != 0x4545ab {
        return stop("stage progression sound switch", target);
    }
    if m.u32(r.wrapping_add(0x104))? != u32::MAX
        && m.u16(r.wrapping_add(0x7e))? == m.u16(r.wrapping_add(8))?
        && m.u8(r.wrapping_add(0xf9))? != 0
    {
        return stop("stage progression sound release", 0x48aac0);
    }
    Ok(())
}

/// The actual table454344 routes all ten lane types to false454869.
pub fn fatigue(m: &MemoriaJogo, kind: u8) -> Result<bool> {
    if !(11..=15).contains(&kind) && !matches!(kind, 20..=24) {
        return stop("stage progression fatigue lane", 0x454869);
    }
    let target = m.u32(0x454344 + 4 * u32::from(kind))?;
    match target {
        0x454851 => Ok(true),
        0x454869 => Ok(false),
        _ => stop("stage progression fatigue switch", target),
    }
}

/// Native454c4f..454c91: only types12 and15 rotate before the state switch.
pub fn before_state(m: &mut MemoriaJogo, member: u16) -> Result<()> {
    let r = unit(m, u32::from(member))?;
    match m.u8(r.wrapping_add(1))? {
        12 | 15 => rotate(m, member),
        // 0x454c6c..0x454c8a: only kinds 12 and 15 rotate; every other kind
        // (including 16..=19 and 25) continues to the state switch untouched.
        _ => Ok(()),
    }
}

/// Full RotateTourEngin4484a7..448598 effects. Native byte wrap and asymmetric
/// half-turn tie are intentional; inputs outside1..32 are not normalized.
pub fn rotate(m: &mut MemoriaJogo, member: u16) -> Result<()> {
    let r = unit(m, u32::from(member))?;
    if !matches!(m.u8(r.wrapping_add(1))?, 12 | 15) {
        return Ok(());
    }
    let wanted = m.u8(r.wrapping_add(0xda))?;
    if wanted == m.u8(r.wrapping_add(0x94))? {
        return Ok(());
    }
    let timer = m.u8(r.wrapping_add(0x96))?.wrapping_add(1);
    m.set_u8(r.wrapping_add(0x96), timer)?;
    if timer != m.u8(r.wrapping_add(0x93))? {
        return Ok(());
    }
    let wanted = m.u8(r.wrapping_add(0xda))?;
    let current = m.u8(r.wrapping_add(0x94))?;
    let increase = if wanted > current && i32::from(wanted) - i32::from(current) < 16 {
        true
    } else {
        current > wanted && i32::from(current) - i32::from(wanted) > 16
    };
    let direction = if increase {
        current % 32 + 1
    } else if current == 1 {
        32
    } else {
        current.wrapping_sub(1)
    };
    m.set_u8(r.wrapping_add(0x94), direction)?;
    m.set_u8(r.wrapping_add(0x96), 0)?;
    Ok(())
}
