//! Focused admission for the original normal-unit experience caller.
//!
//! The sprite, position, palette, fatigue/info order and byte-permutation
//! implementation already live in unit_display_extra and render_units. Keep
//! one implementation; this module supplies the narrowly named adoption API.
use crate::render::{Context, Result};

pub use crate::unit_display_extra::{display, Effects};

/// Only the existing provider's small normal movement callers. This includes
/// its inherited state19 animation frame, which is not bounded by move count.
/// All body effects, bag and health dependencies remain in the reused provider.
pub fn supports(record: &[u8]) -> bool {
    crate::unit_display_extra::supports(record)
}

pub(crate) fn draw(
    context: &mut Context<'_>,
    x: i32,
    y: i32,
    index: u32,
    record: &[u8],
) -> Result<()> {
    context.unit_display_extra(x, y, index, record)
}
