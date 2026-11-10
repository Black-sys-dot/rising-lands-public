//! Focused ordinary-renderer admission for the published terrain0x398 body.
//! No new original code: stage_objects is reused verbatim from 6e6029d8.
use crate::render::{Context, Result};

pub use crate::stage_objects::{facade398 as display, FacadeOutcome};

/// The ordinary caller has checked ownership/discovery and saved its clipping.
/// Keep its restoration/overlay tail and the published body's true dependencies.
pub(crate) fn draw(
    context: &mut Context<'_>,
    x: i32,
    y: i32,
    cell: u32,
    index: u32,
    record: &[u8],
) -> Result<()> {
    context.stage_objects_facade398(x, y, cell, index, record)
}
