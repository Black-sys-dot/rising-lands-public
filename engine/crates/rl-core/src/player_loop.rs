//! _WinMain message loop, supplied only by a declared-input transaction.
use crate::{
    frame::{binding, boundary, stop, EventLog, FrameEvent, FrameInput},
    player::{self, Memory},
    MemoriaJogo, Result,
};
pub fn between(
    m: &mut MemoriaJogo,
    mut input: FrameInput,
) -> Result<(FrameInput, Vec<FrameEvent>, MemoriaJogo)> {
    let divisor = m.b(0x497284)?;
    let drew = m.b(crate::shell_state::MODE)? == 18
        && (m.b(0x4e05a2)? != 0 || divisor != 0 && m.d(0x508f7c)? % divisor == 0);
    let marker = m.w(0x496204)?;
    m.sw(0x496204, 65535)?;
    // The campaign presenter owns shell pixels. These AffGame modes have no
    // gameplay display effects; modes19/24 return without drawing in the original.
    let draw = match m.u8(crate::shell_state::MODE)? {
        0 | 2 | 3 | 12 | 13 | 14 | 16 | 17 | 19 | 23 | 24 => Ok(0),
        _ => crate::frame_input::affgame(m),
    };
    m.sw(0x496204, marker)?;
    let eax = draw?;
    if marker != 65535 && drew {
        if m.b(0x508f7c)? & 1 == 0 {
            m.sw(0x4e05ac, m.w(0x4e05ac)?.wrapping_add(1))?
        }
        if m.w(0x4e05ac)? == 0x1e0a {
            m.sw(0x4e05ac, 0x1e03)?;
            m.sw(0x496204, 65535)?
        }
    }
    if m.b(0x5099af)? != 0 {
        m.sb(0x5099af, 0)?;
        m.sb(0x5099ae, 0)?
    }
    if m.b(0x5099b2)? != 0 {
        m.sb(0x5099b1, 0)?;
        m.sb(0x5099b2, 0)?
    }
    let mut r = input.registers;
    r.eax = eax;
    r.edx = input.saved[0];
    r.ecx = input.saved[1];
    r.ebx = input.saved[2];
    r.esp = r.esp.wrapping_add(24);
    let mut events = EventLog::new(m.audit);
    let messages = m
        .player
        .as_ref()
        .expect("declared runtime")
        .answers
        .messages
        .clone();
    let mut index = 0;
    loop {
        if !m
            .player
            .as_ref()
            .expect("declared runtime")
            .answers
            .display_sound_boundary
        {
            r.eax = crate::player_audio::manage(m)?;
        }
        boundary(
            m,
            &mut events,
            103,
            0x460220,
            [r.eax, r.edx, r.ebx, r.ecx],
            input.msg,
            15,
        )?;
        boundary(
            m,
            &mut events,
            104,
            0x460225,
            [r.eax, r.edx, r.ebx, r.ecx],
            input.msg,
            15,
        )?;
        if r.ebx as u8 != m.u8(0x49a059)? {
            return stop("_WinMain@16 quit path", 0x460227);
        }
        let message = messages.get(index).copied();
        let mut v = vec![u32::from(message.is_some())];
        v.extend(message.unwrap_or([0; 3]));
        player::event(m, 1, 0x4601d8, &v);
        binding(
            &mut events,
            6,
            0x4601d8,
            &[r.esp, r.ebx, r.ebx, r.ebx, r.ebx],
            u32::from(message.is_some()),
        );
        let Some(msg) = message else {
            r.eax = 0;
            break;
        };
        index += 1;
        let hwnd = m.d(0x4e27e8)?;
        player::event(m, 2, 0x4601eb, &msg);
        binding(&mut events, 9, 0x4601eb, &[r.esp, 0, 0, 0], 1);
        binding(&mut events, 10, 0x4601f9, &[r.esp], 0);
        player::event(m, 3, 0x4601f9, &[0]);
        binding(&mut events, 11, 0x460203, &[r.esp], 0);
        let (wp, lp) = crate::player_input::wnd(m, msg)?;
        let mut flags = [0; 12];
        flags[..3].copy_from_slice(&msg);
        flags[3] = hwnd;
        let bytes = m.read(0x5099a8, 11)?;
        for (i, chunk) in bytes.chunks(4).enumerate() {
            let mut b = [0; 4];
            b[..chunk.len()].copy_from_slice(chunk);
            flags[4 + i] = u32::from_le_bytes(b)
        }
        player::event(m, 4, 0x460203, &flags);
        r.eax = 0;
        r.ecx = lp;
        r.edx = wp;
        input.msg = [hwnd, msg[0], msg[1], msg[2]];
    }
    if r.ebx as u8 == m.u8(0x49a058)? {
        return stop("_WinMain@16 WaitMessage path", 0x460214);
    }
    input.saved = [r.edx, r.ecx, r.ebx];
    r.esp = r.esp.wrapping_sub(24);
    let counter = m.d(0x508f7c)?.wrapping_add(1);
    m.sd(0x508f7c, counter)?;
    if counter % 2 == 0 {
        m.sd(0x508f80, m.d(0x508f80)?.wrapping_add(1))?
    }
    if counter % 3 == 0 {
        m.sd(0x508f84, m.d(0x508f84)?.wrapping_add(1))?
    }
    r.ebx = 3;
    r.edx = r.esp;
    binding(&mut events, 7, 0x47504b, &[r.esp], 1);
    player::event(
        m,
        5,
        0x47504b,
        &[input.cursor_x as u32, input.cursor_y as u32],
    );
    m.sw(0x5099a8, input.cursor_x as u32)?;
    m.sw(0x5099aa, input.cursor_y as u32)?;
    r.eax = input.cursor_y as u32;
    r.esp = r.esp.wrapping_sub(4);
    input.registers = r;
    Ok((input, events.into_vec(), MemoriaJogo::default()))
}
