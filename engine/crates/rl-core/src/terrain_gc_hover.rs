//! InputGame GC hover continuation, intact caller 0x4299e4..0x429a5b.
//! The caller owns the hover local: a negative query leaves it unchanged and
//! continues at 0x4299fe; a positive query stores the real message-table result
//! and jumps to 0x429a5b, past the mine and pit checks.
use crate::{terrain_gc, text, MemoriaJogo, Result};

pub fn lookup(m: &MemoriaJogo, cell: u32) -> Result<Option<u32>> {
    if terrain_gc::terrain_est_gc(m, cell)? as u8 == 0 {
        Ok(None)
    } else {
        Ok(Some(text::get(m, 44)?))
    }
}

/// Original 0x42c2ae..0x42c309, before the conditional NextGouttes call.
/// Inputs are the caller locals at EBP-0x34, EBP-0x14 and EBP-8.
/// No text is dereferenced here. A null message pointer is a real table value.
pub fn publish(m: &mut MemoriaJogo, info: u32, info2: u32, delayed: bool) -> Result<()> {
    if m.u32(0x4e060c)? != info {
        m.set_u16(0x4e0610, if delayed { 1 } else { m.u16(0x49620a)? })?;
    } else {
        let timer = m.u16(0x4e0610)?;
        if timer < m.u16(0x49620a)? {
            m.set_u16(0x4e0610, timer.wrapping_add(1))?;
        }
    }
    m.set_u32(0x4e060c, info)?;
    m.set_u16(0x49620c, info2 as u16)
}
