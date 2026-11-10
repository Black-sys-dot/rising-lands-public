//! Original menu state primitives (0x469320 family), MenuBefore and result latch.
//! Original-address memory is supplied by the caller; no RNG or host effects.
use crate::{EngineError, FrameRegisters, MemoriaJogo, Result};

pub const COUNT: u32 = 0x4e2f40;
pub const ITEMS: u32 = 0x4e2f44;
pub const SELECTED: u32 = 0x4e2f6c;
pub const TITLE: u32 = 0x4e2f70;

pub fn scroll(m: &mut MemoriaJogo, direction: i8) -> Result<()> {
    let selected = m.u16(SELECTED)?;
    if direction == -1 && selected != 0 {
        m.set_u16(SELECTED, selected.wrapping_sub(1))?;
    } else if direction == 1 && u32::from(selected) != u32::from(m.u16(COUNT)?).wrapping_sub(1) {
        m.set_u16(SELECTED, selected.wrapping_add(1))?;
    }
    Ok(())
}

pub fn reset(m: &mut MemoriaJogo) -> Result<()> {
    m.set_u32(TITLE, 0)?;
    m.set_u16(COUNT, 0)
}

pub fn add_item(m: &mut MemoriaJogo, text: u32) -> Result<()> {
    let count = m.u16(COUNT)?;
    m.set_u16(COUNT, count.wrapping_add(1))?;
    m.set_u32(ITEMS + u32::from(count) * 4, text)?;
    m.set_u16(SELECTED, 0)
}

/// MenuBefore 0x42c776, including the original identity cases.
pub fn before(menu: u8) -> u8 {
    match menu {
        3 | 4 | 10 | 17 => 2,
        5 => 4,
        6..=9 => 5,
        12 | 14 | 18 | 23 => 16,
        13 => 3,
        16 => 18,
        _ => menu,
    }
}

/// Register-level entry for routine differentials. Game callers can use the
/// typed functions; ESP models RET and the other seven data registers are exact.
pub fn entry(m: &mut MemoriaJogo, address: u32, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let ax = |eax: u32, value: u16| (eax & 0xffff0000) | u32::from(value);
    match address {
        0x469320 => {
            let selected = m.u16(SELECTED)?;
            if r.eax as i8 == -1 && selected != 0 {
                m.set_u16(SELECTED, selected.wrapping_sub(1))?;
            } else {
                r.eax = r.eax as i8 as i32 as u32;
                if r.eax == 1 {
                    r.eax = u32::from(m.u16(COUNT)?).wrapping_sub(1);
                    scroll(m, 1)?;
                }
            }
        }
        0x46936d => r.eax = ax(r.eax, m.u16(COUNT)?),
        0x469374 => {
            r.eax &= !0xff00;
            m.set_u16(SELECTED, r.eax as u16)?;
        }
        0x46937d => r.eax = ax(r.eax, m.u16(SELECTED)?),
        0x469384 => {
            let text = r.eax;
            r.eax = u32::from(m.u16(SELECTED)?);
            m.set_u32(ITEMS + r.eax * 4, text)?;
        }
        0x469398 => m.set_u32(TITLE, r.eax)?,
        0x46939e => {
            let text = r.eax;
            r.eax = u32::from(m.u16(COUNT)?);
            add_item(m, text)?;
        }
        0x4693c1 => reset(m)?,
        0x4693d5 => {
            m.set_u8(0x4e2f74, r.eax as u8)?;
            m.set_u8(0x4e2f75, r.edx as u8)?;
            m.set_u8(0x4e2f76, r.ebx as u8)?;
        }
        0x42c776 => r.eax = (r.eax & !255) | u32::from(before(r.eax as u8)),
        0x4585e4 => r.eax = (r.eax & !255) | u32::from(m.u8(0x4e1d84)?),
        0x47c6b9 => {
            let i = (r.eax & 65535) + u32::from(m.u16(0x511ff8)?);
            r.eax = m.u32(0x511828 + 4 * i)?;
        }
        0x47c7ca => {
            r.eax &= !0xff00;
            m.set_u8(0x511ffa, 0)?;
            r.eax = r.eax as i8 as i32 as u32;
            if (r.eax as i32) < 0 {
                r.eax = u32::from(m.u16(0x511ff8)?);
                if r.eax != 0 {
                    m.set_u8(0x511ffa, 1)?;
                    m.set_u16(0x511ff8, (r.eax as u16).wrapping_sub(18))?;
                }
            } else if r.eax != 0 {
                r.eax = u32::from(m.u16(0x511ff8)?) + 18;
                if r.eax < u32::from(m.u16(0x511824)?) {
                    m.set_u8(0x511ffa, 1)?;
                    m.set_u16(0x511ff8, r.eax as u16)?;
                }
            }
        }
        _ => return Err(EngineError::UnknownRoutine(address)),
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
