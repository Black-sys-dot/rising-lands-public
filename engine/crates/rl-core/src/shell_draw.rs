//! MenuDisplay0x4693e7 and AffObjectif0x47c6d3, using the existing bitmap renderer.
use crate::{
    render::{Context, Error, Renderer, Result},
    shell_briefing, shell_menu, FrameRegisters, MemoriaJogo,
};
use std::collections::BTreeMap;

fn context<'a>(renderer: &'a mut Renderer, m: &'a MemoriaJogo, clip: [i32; 4]) -> Context<'a> {
    Context {
        background: &mut renderer.saved_background,
        text_boundaries: &mut renderer.recorded_text_boundaries,
        text_checked: 0,
        tick: renderer.tick,
        m,
        overrides: BTreeMap::new(),
        surface: &mut renderer.surface,
        cache: &mut renderer.cache,
        assets: &renderer.assets,
        observations: Vec::new(),
        audit: renderer.audit,
        clip,
        camera: 0,
    }
}
fn clip(m: &MemoriaJogo) -> Result<[i32; 4]> {
    Ok([
        i32::from(m.u16(0x49a020)?),
        i32::from(m.u16(0x49a01c)?),
        i32::from(m.u16(0x49a022)?),
        i32::from(m.u16(0x49a01e)?),
    ])
}
fn text(
    c: &mut Context<'_>,
    pointer: u32,
    rect: [i32; 4],
    font: u32,
    clipped: bool,
) -> Result<u32> {
    let raw = shell_briefing::cstring(c.m, pointer, 129)?;
    let end = raw.iter().position(|&b| b == 10).unwrap_or(raw.len());
    let terminator = raw.get(end).copied().unwrap_or(0);
    let mut line = raw[..end].to_vec();
    line.push(terminator);
    c.afftext(&line, rect, font, [0, u32::from(clipped)])?;
    Ok(u32::from(terminator))
}

fn string_line(
    c: &mut Context<'_>,
    pointer: u32,
    mut x: i32,
    y: i32,
    clipped: bool,
) -> Result<(u32, usize)> {
    if c.u32(0x4e1d94)? != 0 || c.u32(0x4e1d98)? != 0 {
        return c.stop(0x4591fe, "AffString external font mode");
    }
    let raw = shell_briefing::cstring(c.m, pointer, 129)?;
    let end = raw.iter().position(|&b| b == 10).unwrap_or(raw.len());
    if end > 127 {
        return c.stop(0x4591e1, "AffString signed cursor overflow");
    }
    for &byte in &raw[..end] {
        let id = 0x23d5 + u32::from(byte.wrapping_sub(32));
        if byte != 32 {
            c.bob(x as i16 as i32, y, id, clipped, None)?;
        }
        let metrics = 0x23d5
            + u32::from(if byte == 32 {
                b'l'.wrapping_sub(32)
            } else {
                byte.wrapping_sub(32)
            });
        x = x.wrapping_add(c.assets.sprite(metrics)?.width as i32);
    }
    Ok((u32::from(raw.get(end).copied().unwrap_or(0)), end))
}

/// AffString0x4591c8 differs from AffText for control bytes: the unsigned
/// glyph subtraction wraps, including the DOS EOF byte in original briefings.
pub fn string(
    renderer: &mut Renderer,
    m: &MemoriaJogo,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    let pointer = r.ebx;
    let mut c = context(renderer, m, clip(m)?);
    let (terminator, length) = string_line(
        &mut c,
        pointer,
        r.eax as i32,
        r.edx as i16 as i32,
        r.ecx as u8 != 0,
    )?;
    r.eax = terminator;
    r.edx = pointer + length as u32;
    if length != 0 {
        r.ebx = r.esp.wrapping_sub(40);
        r.ecx = r.esp.wrapping_sub(32);
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

pub fn menu(
    renderer: &mut Renderer,
    m: &MemoriaJogo,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    let count = m.u16(shell_menu::COUNT)?;
    if count != 0 {
        let width = i32::from(m.u16(0x49a018)?);
        let mut c = context(renderer, m, clip(m)?);
        text(
            &mut c,
            m.u32(shell_menu::TITLE)?,
            [0, 246, width, 276],
            u32::from(m.u8(0x4e2f74)?),
            false,
        )?;
        for i in 0..count {
            let font = m.u8(if i == m.u16(shell_menu::SELECTED)? {
                0x4e2f76
            } else {
                0x4e2f75
            })?;
            text(
                &mut c,
                m.u32(shell_menu::ITEMS + u32::from(i) * 4)?,
                [0, 286 + 25 * i32::from(i), width, 306 + 25 * i32::from(i)],
                u32::from(font),
                false,
            )?;
        }
        // The last loop increment remains in EAX, after the void AffText calls.
        r.eax = 286 + 25 * u32::from(count);
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

pub fn objectives(
    renderer: &mut Renderer,
    m: &mut MemoriaJogo,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    if m.u8(shell_briefing::DIRTY)? != 0 {
        if m.u32(0x4e1d94)? != 0 || m.u32(0x4e1d98)? != 0 {
            return Err(Error::Stop(
                0x47c738,
                "AffObjectif external font mode".into(),
            ));
        }
        let saved = clip(m)?;
        for (address, value) in [0x49a02c, 0x49a02e, 0x49a030, 0x49a032]
            .into_iter()
            .zip([saved[0], saved[2], saved[1], saved[3]])
        {
            m.set_u16(address, value as u16)?;
        }
        for (address, value) in [
            (0x49a020, 55),
            (0x49a022, 310),
            (0x49a01c, 212),
            (0x49a01e, 486),
        ] {
            m.set_u16(address, value)?;
        }
        let mut c = context(renderer, m, [55, 212, 310, 486]);
        c.filled(55, 212, 310, 486, 42)?;
        let page = u32::from(m.u16(shell_briefing::PAGE)?);
        let count = m.u16(shell_briefing::LINE_COUNT)?;
        let mut x = r.edi as i32;
        for row in 0..18u16 {
            let pointer = m.u32(shell_briefing::LINES + 4 * (page + u32::from(row)))?;
            if row <= count {
                let line = shell_briefing::cstring(m, pointer, 129)?;
                let mut width = 0u16;
                for &byte in line.iter().take_while(|&&b| b != 10) {
                    width = width.wrapping_add(
                        c.assets
                            .sprite(
                                0x23d5
                                    + u32::from(
                                        if byte == 32 { b'l' } else { byte }.wrapping_sub(32),
                                    ),
                            )?
                            .width as u16,
                    );
                }
                x = ((255 - i32::from(width)) / 2).max(0);
            }
            let y = 212 + i32::from(m.u16(0x498408)?) * i32::from(row);
            string_line(&mut c, pointer, 55 + x, y, true)?;
        }
        for (address, value) in [0x49a020, 0x49a022, 0x49a01c, 0x49a01e]
            .into_iter()
            .zip([saved[0], saved[2], saved[1], saved[3]])
        {
            m.set_u16(address, value as u16)?;
        }
        r.eax = saved[3] as u32;
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Prepared MenuInit result caption block0x468b2b, after its picture load.
/// Returns the exact GPRs at0x468c60; resource/platform initialization is separate.
pub fn result_caption_block(
    renderer: &mut Renderer,
    m: &MemoriaJogo,
    r: FrameRegisters,
) -> Result<FrameRegisters> {
    renderer.afftext_fixture(
        m,
        crate::text::get(m, 116)?,
        3,
        [0, 0],
        FrameRegisters {
            eax: 0,
            edx: 145,
            ebx: 450,
            ecx: 215,
            esp: r.esp.wrapping_sub(20),
            ..r
        },
    )
}

/// Prepared MenuInit briefing/statistics draw block0x468614..0x468821.
/// RechercheCalculNew is a prerequisite of mode0x11, not a display no-op.
pub fn research_block(
    renderer: &mut Renderer,
    m: &MemoriaJogo,
    player: u8,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    let base = 0x5080f4 + 16 * u32::from(player);
    let color =
        crate::frame_economy::call(&mut m.clone(), "PlayerGetColor", &[u32::from(player)])? as u8;
    let mut c = context(renderer, m, [0, 0, 799, 599]);
    for (column, branch) in [0, 2, 1, 3].into_iter().enumerate() {
        let level = m.u8(base + branch)?;
        for index in 0..level.min(20) {
            let left = 442 + 103 * column as i32;
            let top = 425 - 15 * i32::from(index);
            c.filled(left, top, left + 4, top + 10, color)?;
        }
    }
    let progress = f64::from_le_bytes(m.read(base + 8, 8)?.try_into().unwrap());
    if !progress.is_finite()
        || progress.trunc() < f64::from(i32::MIN)
        || progress.trunc() > f64::from(i32::MAX)
    {
        return c.stop(
            0x468760,
            "MenuInit research progress outside finite x87 integer domain",
        );
    }
    let branch = u32::from(m.u8(base + 4)?);
    let level = u32::from(m.u8(base + branch)?);
    let cost = m.u32(0x50817c + 360 * branch + 12 * level)? as u16;
    if cost == 0 {
        return c.stop(0x4687b2, "MenuInit research percentage division by zero");
    }
    // __CHP0x48c43e changes RC to chop for FRNDINT, then restores the x87 CW.
    let percentage = ((u32::from(progress.trunc() as i32 as u16) * 100) / u32::from(cost)) as u16;
    let text = format!("{percentage}%\0").into_bytes();
    let pointer = r.esp.wrapping_add(0x100);
    let mut local = m.clone();
    local.declare(pointer, text)?;
    r.esi = 0x494a8e;
    r.edi = pointer;
    renderer.afftext_fixture(
        &local,
        pointer,
        1,
        [0, 0],
        FrameRegisters {
            eax: 680,
            edx: 51,
            ebx: 735,
            ecx: 87,
            esp: r.esp.wrapping_sub(20),
            ..r
        },
    )
}
