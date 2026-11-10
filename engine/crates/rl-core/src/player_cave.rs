//! Economy cave body, pinned main54d148d, caverna_recrutamento.py:corpo_caverna.
//! Original routine0x426aff, full caller0x4268d4; only declared bytes are read.
use crate::{
    frame::stop,
    player::{structure, Memory},
    FrameRegisters, MemoriaJogo, Result,
};
pub fn body(m: &mut MemoriaJogo, s: u32) -> Result<u32> {
    let e = structure(s);
    if m.b(e + 8)? != 3 {
        return stop("BatimentNextEtape type-3 body", 0x426aff);
    }
    let bag = e + 12;
    let mut count = 0u32;
    for i in 0..m.w(bag + 2)? {
        if m.b(bag + 0x42 + i)? == 1 {
            let u = m.w(bag + 6 + 2 * i)?;
            if m.b(m.d(0x4e19a0)?.wrapping_add(u * 276 + 1))? == 9 {
                count = count.wrapping_add(1)
            }
        }
    }
    if count & 65535 == 0 {
        return Ok(count & !255);
    }
    if m.b(0x508f80)? & 1 != 0 {
        return Ok(3);
    }
    let counter = m.b(e + 0x76)?;
    if m.b(e + 0x75)? != 0 {
        if counter < 15 {
            m.sb(e + 0x76, counter.wrapping_add(1))?
        }
        return Ok(counter);
    }
    if counter == 0 {
        return Ok(0);
    }
    m.sb(e + 0x76, counter.wrapping_sub(1))?;
    Ok(((counter.wrapping_sub(1) & 255) << 8) | counter)
}
pub fn entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    r.eax = body(m, r.eax)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
