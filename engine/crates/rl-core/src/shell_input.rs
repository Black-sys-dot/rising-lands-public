//! CalculGame shell input blocks. Keys are completed press/release events;
//! platform polling/holding belongs to the later app adapter. Mouse flags retain
//! their original meanings at0x5099ae/af. No gameplay/RNG execution is hidden.
use crate::{frame::stop, shell_menu, shell_state, FrameRegisters, MemoriaJogo, Result};
#[derive(Clone, Copy, Default)]
pub struct Keys {
    pub enter: bool,
    pub up: bool,
    pub down: bool,
    pub escape: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Boundary {
    pub registers: FrameRegisters,
    pub address: u32,
}
const END: u32 = 0x42d4e2;
fn menu_request(m: &MemoriaJogo, mut r: FrameRegisters, target: u8) -> Result<Boundary> {
    r.eax = u32::from(m.u8(0x508041)?);
    r.edx = target as i8 as i32 as u32;
    r.esp = r.esp.wrapping_sub(4);
    Ok(Boundary {
        registers: r,
        address: 0x4682bb,
    })
}
fn activate(m: &MemoriaJogo, mut r: FrameRegisters) -> Result<Boundary> {
    r.ebx = u32::from(m.u16(shell_menu::SELECTED)?);
    r.edx = u32::from(m.u8(shell_state::MODE)?);
    r.eax = u32::from(m.u8(0x508041)?);
    r.esp = r.esp.wrapping_sub(4);
    Ok(Boundary {
        registers: r,
        address: 0x42c3e4,
    })
}
pub fn menu(m: &mut MemoriaJogo, mut r: FrameRegisters, keys: Keys) -> Result<Boundary> {
    let mode = m.u8(shell_state::MODE)?;
    if !matches!(mode, 2 | 3 | 12 | 13 | 14 | 16) {
        return stop("CalculGame selection input mode", 0x42cfe3);
    }
    if mode == 16 && m.u8(0x4e0730)? != 0 {
        return stop("CalculGame network pause continues gameplay", 0x42d15a);
    }
    if keys.enter {
        return activate(m, r);
    }
    if keys.up {
        shell_menu::scroll(m, -1)?;
    } else if keys.down {
        shell_menu::scroll(m, 1)?;
    } else if keys.escape {
        return menu_request(m, r, shell_menu::before(m.u8(shell_state::MODE)?));
    }
    r.eax = crate::player_input::zone(m)?;
    m.set_u16(0x5099ac, r.eax as u16)?;
    if r.eax as u16 != 0 {
        r.eax = u32::from((r.eax as u8).wrapping_sub(1));
        m.set_u16(shell_menu::SELECTED, r.eax as u16)?;
    }
    if m.u8(0x5099ae)? != 0 && m.u8(0x5099af)? != 0 {
        return activate(m, r);
    }
    if mode == 16 {
        r.eax &= !255;
    }
    Ok(Boundary {
        registers: r,
        address: END,
    })
}

pub fn briefing(m: &mut MemoriaJogo, mut r: FrameRegisters, keys: Keys) -> Result<Boundary> {
    let mode = m.u8(shell_state::MODE)?;
    if !matches!(mode, 17 | 23) {
        return stop("CalculGame briefing input mode", 0x42caa9);
    }
    if m.u8(0x5099ae)? != 0 {
        let zone = crate::player_input::zone(m)?;
        m.set_u16(0x5099ac, zone as u16)?;
        if mode == 17 && (1..=4).contains(&zone) {
            let branch = match zone {
                1 => 0,
                2 => 2,
                3 => 1,
                _ => 3,
            };
            let player = u32::from(m.u8(0x508041)?);
            m.set_u8(0x5080f8 + 16 * player, branch)?;
            r.edx = u32::from(branch);
        } else if mode == 17 && zone == 5 {
            m.set_u8(shell_state::MODE, 0)?;
        } else if zone == 6 || zone == 7 {
            let counter = m.u32(0x508f7c)?;
            r.ebx = 10;
            r.edx = counter % 10;
            if r.edx == 0 {
                shell_menu::entry(
                    m,
                    0x47c7ca,
                    FrameRegisters {
                        eax: if zone == 6 { u32::MAX } else { 1 },
                        ..r
                    },
                )?;
            }
        } else if (zone == 8 || (mode == 23 && zone == 5)) && m.u8(0x5099af)? != 0 {
            return menu_request(m, r, shell_menu::before(mode));
        }
    }
    if keys.enter {
        if mode == 23 {
            return menu_request(m, r, shell_menu::before(mode));
        }
        r.eax = 0xffff8000;
        r.edx &= !255;
        m.set_u8(shell_state::MODE, 0)?;
    } else {
        r.eax = 0;
    }
    Ok(Boundary {
        registers: r,
        address: END,
    })
}

/// Original result latch branch before failure-video lookup / victory input.
pub fn result_gate(m: &MemoriaJogo, mut r: FrameRegisters) -> Result<Boundary> {
    r.eax = u32::from(m.u8(0x4e1d84)?);
    Ok(Boundary {
        registers: r,
        address: if r.eax == 0 { 0x42c9d5 } else { 0x42ca38 },
    })
}
pub fn victory(m: &MemoriaJogo, mut r: FrameRegisters, keys: Keys) -> Result<Boundary> {
    r.eax = 0;
    if keys.enter || (m.u8(0x5099ae)? != 0 && m.u8(0x5099af)? != 0) {
        r.esp = r.esp.wrapping_sub(4);
        Ok(Boundary {
            registers: r,
            address: 0x46c08c,
        })
    } else {
        Ok(Boundary {
            registers: r,
            address: END,
        })
    }
}
