//! EnginNextEtape states 3 (0x455a61) and 6 (0x4568ad), transcribed from
//! intact x86.
use crate::player::{alive, unit, Memory};
use crate::{MemoriaJogo, Result};

fn release_focus(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    if u == m.w(0x496304)? {
        m.sb(0x4e0f36, 0)?;
        m.sb(0x4e0f37, 0)?;
        m.sd(0x4e0f3c, 0)?;
        m.sw(0x496304, 0xffff)?;
    }
    Ok(())
}

/// A live unit falls back to state 7 (0x455ad1, 0x456900).
fn idle(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    if alive(m, u)? {
        m.sb(unit(m, u)? + 2, 7)?;
        release_focus(m, u)?;
    }
    Ok(())
}

/// State 3 (0x455a61): turn until the heading reaches its target, then idle.
pub fn state3(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let u = u32::from(u);
    let r = unit(m, u)?;
    if m.b(r + 0x90)? != m.b(r + 0x91)? {
        crate::frame_movement::call(m, "RotateEngin", &[u])?;
        return Ok(());
    }
    idle(m, u)
}

/// State 6 (0x4568ad): idle.
pub fn state6(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    idle(m, u32::from(u))
}
