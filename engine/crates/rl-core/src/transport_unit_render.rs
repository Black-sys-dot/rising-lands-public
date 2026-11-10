//! Narrow state18 adoption of the existing intact movement display body.
use crate::render::{Context, Renderer, Result};
pub use crate::unit_display_extra::Effects;
use crate::{FrameRegisters, MemoriaJogo};

/// Only the actual owner0 ground/storage transport carrier, without effects or jitter.
/// There is no invented phase/count bound: native uses the low-word sprite ID.
pub fn supports(r: &[u8]) -> bool {
    r.len() == 276
        && r[0] == 0
        && r[1] == 5
        && r[2] == 18
        && matches!(r[0x82], 1 | 2)
        && r[0x99] == 0
        && r[0xdc] == 0
}

pub fn display(renderer: &mut Renderer, m: &MemoriaJogo, r: FrameRegisters) -> Result<Effects> {
    let record = m.read(m.u32(0x4e19a0)?.wrapping_add(276 * (r.ebx & 65535)), 276)?;
    if !supports(record) || m.u8(0x508041)? != 0 {
        return Err(crate::render::Error::Stop(
            0x41591f,
            "transport display outside proved owner0 type5 state18 body".into(),
        ));
    }
    crate::unit_display_extra::display(renderer, m, r)
}

pub(crate) fn draw(c: &mut Context<'_>, x: i32, y: i32, index: u32, r: &[u8]) -> Result<()> {
    if !supports(r) || c.camera != 0 {
        return c.stop(
            0x41591f,
            "transport display outside proved owner0 type5 state18 body",
        );
    }
    c.unit_display_extra(x, y, index, r)
}
