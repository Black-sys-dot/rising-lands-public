//! Original MenuInit item-building blocks and shell hit zones. Platform resource
//! ownership, campaign reset/loading and video/audio are explicit caller actions.
use crate::{frame::stop, shell_menu, text, FrameRegisters, MemoriaJogo, Result};
pub const MODE: u32 = 0x4e2f78;

pub fn configure(m: &mut MemoriaJogo, mode: u8) -> Result<()> {
    let items: &[u32] = match mode {
        2 => &[32, 33, 34, 35, 36],
        3 => &[37, 38],
        16 => &[5, 6, 10, 20, 11, 12],
        _ => return stop("MenuInit item-building mode", 0x468ca7),
    };
    shell_menu::reset(m)?;
    m.set_u32(shell_menu::TITLE, text::get(m, 31)?)?;
    for &id in items {
        shell_menu::add_item(m, text::get(m, id)?)?;
    }
    m.set_u8(0x4e2f74, 5)?;
    m.set_u8(0x4e2f75, 1)?;
    m.set_u8(0x4e2f76, 2)?;
    Ok(())
}

/// Register state at the original InitGameZone call boundary. This is a scoped
/// MenuInit block, not a replacement for its Win32/resource/campaign callees.
pub fn configure_registers(
    m: &mut MemoriaJogo,
    mode: u8,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    configure(m, mode)?;
    r.eax = 3;
    r.ebx = 0;
    r.ecx = 0;
    r.edx = 0;
    r.esi = 0;
    if mode == 3 {
        r.edi = 0;
    } else {
        r.ebp = 0;
    }
    r.esp = r.esp.wrapping_sub(4);
    Ok(r)
}

fn zone(m: &mut MemoriaJogo, id: u16, rect: [u16; 4]) -> Result<()> {
    for (i, v) in rect.into_iter().enumerate() {
        m.set_u32(0x5099b4 + u32::from(id) * 16 + 4 * i as u32, u32::from(v))?;
    }
    if id > m.u16(0x50a9b8)? {
        m.set_u16(0x50a9b8, id)?;
    }
    Ok(())
}

/// InitGameZone menu mode3 and briefing mode2; game mode1 uses the existing port.
pub fn zones(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    match r.eax as u16 {
        2 => {
            let mut rectangles = [
                [402, 486, 480, 547],
                [506, 473, 579, 555],
                [620, 471, 691, 564],
                [727, 468, 776, 545],
                [290, 512, 334, 540],
                [270, 162, 289, 182],
                [299, 162, 321, 182],
                [66, 45, 126, 87],
            ];
            if m.u8(0x496fe4)? < 7 {
                rectangles[3] = [0, 0, 0, 1];
            }
            for (i, rect) in rectangles.into_iter().enumerate() {
                zone(m, i as u16 + 1, rect)?;
            }
            r.ebx = 45;
            r.ecx = 126;
            r.edx = 8;
        }
        3 => {
            let count = m.u16(shell_menu::COUNT)?;
            if count > 255 {
                return stop("InitGameZone native byte counter cannot finish", 0x4719ad);
            }
            for i in 0..count {
                let y = 286u16.wrapping_add(25u16.wrapping_mul(i));
                zone(m, i + 1, [0, y, m.u16(0x49a018)?, y.wrapping_add(20)])?;
                r.ebx = u32::from(y & 255) | u32::from((i + 1) & 255) << 8;
                r.ecx = u32::from(m.u16(0x49a018)?);
            }
            r.edx = u32::from(count);
        }
        _ => return stop("InitGameZone shell mode", 0x471830),
    }
    m.set_u16(0x50a9b4, 1)?;
    m.set_u16(0x50a9b6, r.edx as u16)?;
    r.eax = 1;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Menu(u8),
    /// MissionReset0x46c084 followed by player initialization and this screen.
    ResetCampaign {
        next_screen: u8,
    },
    /// Lists12 (in game) and13 (main menu): slot path0x46b308, FileExists,
    /// GameLoad0x46b88d, then MenuInit(18). The host owns the saved games.
    LoadSlot {
        slot: u8,
        from_menu: bool,
    },
    /// List14: GameSave0x46bd45 on the slot, then MenuInit(18).
    SaveSlot {
        slot: u8,
    },
    None,
}

/// MenuNewSelect's main, single-player and pause dispatch, stopping at its first
/// explicit MenuInit/MissionReset dependency. No dependency is executed or hidden.
pub fn selection(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<(FrameRegisters, Action)> {
    let player = r.eax as u8;
    let mode = r.edx as u8;
    let selected = r.ebx as u16;
    r.eax = r.ebx;
    let action = match mode {
        2 => {
            r.edx = r.ebx;
            if selected > 4 {
                Action::None
            } else {
                Action::Menu([3, 4, 1, 10, 21][usize::from(selected)])
            }
        }
        3 => {
            m.set_u8(0x4e0730, 0)?;
            r.edx = r.ebx;
            match selected {
                0 => Action::ResetCampaign { next_screen: 17 },
                1 => Action::ResetCampaign { next_screen: 13 },
                _ => Action::None,
            }
        }
        12 | 13 => {
            r.esi = u32::from(selected as u8);
            r.eax = r.esi;
            Action::LoadSlot {
                slot: selected as u8,
                from_menu: mode == 13,
            }
        }
        14 => {
            r.eax = u32::from(selected as u8);
            Action::SaveSlot {
                slot: selected as u8,
            }
        }
        16 => {
            if selected > 4 {
                Action::Menu(18)
            } else {
                r.eax &= 65535;
                let network = m.u8(0x4e0730)?;
                if selected != 3 && network != 0 {
                    r.eax = u32::from(network);
                    Action::None
                } else {
                    Action::Menu([12, 14, 23, 2, 17][usize::from(selected)])
                }
            }
        }
        _ => return stop("MenuNewSelect shell dispatch mode", 0x42c3e4),
    };
    match action {
        Action::Menu(target) => {
            r.eax = u32::from(player);
            r.edx = u32::from(target);
            r.esp = r.esp.wrapping_sub(16);
        }
        Action::ResetCampaign { .. } | Action::LoadSlot { .. } | Action::SaveSlot { .. } => {
            r.esp = r.esp.wrapping_sub(16)
        }
        Action::None => r.esp = r.esp.wrapping_add(4),
    }
    Ok((r, action))
}
