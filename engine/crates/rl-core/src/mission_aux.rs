//! Original auxiliary ETA loaders 0x47534a..0x47ac30 in MissionLoad order.
use crate::{
    frame_core,
    mission_ini::MissionIni,
    mission_load::{fill, require, Reader},
    MemoriaJogo, Result,
};
pub(crate) fn monster_init(m: &mut MemoriaJogo, ini: &MissionIni) -> Result<()> {
    for i in 0..100 {
        let a = 0x50a9bc + 20 + 12 * i;
        m.set_u32(a, u32::MAX)?;
        m.set_u16(a + 4, 0)?;
        m.set_u32(a + 8, 0)?;
        let r = frame_core::random(m)?;
        m.set_u8(a + 6, (r & 15) as u8)?;
    }
    m.set_u8(0x50a9bc, 0)?;
    if ini.integer(b"GENERAL", b"HumanOnly", 0) != 0 {
        m.set_u8(0x50a9bc, 1)?;
    }
    for (key, default, offset) in [
        (b"ProbaCaveJoueur".as_slice(), 5, 4),
        (b"ProbaCaveIA", 2, 8),
        (b"ProbaEmbuscade", 2, 12),
    ] {
        m.set_u32(0x50a9bc + offset, ini.integer(b"GENERAL", key, default))?;
    }
    fill(m, 0x50af84, 432, 0)?;
    for g in 0..16 {
        let key = format!("GROUPE{g:02}");
        let text = ini.string(b"GENERAL", key.as_bytes(), b"", 256);
        if text.is_empty() {
            continue;
        }
        for part in text.split(|c| *c == b',') {
            let text = String::from_utf8_lossy(part);
            // Shipped MONSTRE.INI uses decimal %i tokens. Reject unsupported forms explicitly.
            let text = text.trim_start();
            let end = text
                .char_indices()
                .find(|(i, c)| !c.is_ascii_digit() && !(*i == 0 && matches!(c, '+' | '-')))
                .map_or(text.len(), |(i, _)| i);
            if end == 0 {
                continue;
            }
            let Ok(index) = text[..end].parse::<i32>() else {
                continue;
            };
            require(
                (0..8).contains(&index),
                "InitEntrerMonstre group type",
                0x47a7c9,
            )?;
            let t = u32::from(m.u8(0x498400 + index as u32)?);
            let a = 0x50af84 + 26 * g + t;
            m.set_u8(a, m.u8(a)?.wrapping_add(1))?;
            let a = 0x50b124 + g;
            m.set_u8(a, m.u8(a)?.wrapping_add(1))?;
        }
    }
    Ok(())
}
pub(crate) fn load(m: &mut MemoriaJogo, r: &mut Reader<'_>, ini: &MissionIni) -> Result<()> {
    let n = u32::from(r.word()?);
    require(n <= 20, "LoadMine count", 0x47534a)?;
    for i in 0..n {
        r.read(m, 0x509954 + 4 * i, 4)?;
    }
    for i in n..20 {
        m.set_u16(0x509954 + 4 * i, 0)?;
    }
    m.set_u32(0x50d99c, 0)?;
    for i in 0..100 {
        let a = 0x50d5b4 + 10 * i;
        m.set_u8(a, 200)?;
        fill(m, a + 1, 7, 255)?;
        m.set_u8(a + 8, 0)?;
    }
    let n = u32::from(r.word()?);
    let size = usize::from(r.word()?);
    require(n <= 100 && size == 8, "IALoadAttaqueIni header", 0x47c24d)?;
    for i in 0..n {
        r.read(m, 0x50d5b4 + 10 * i, size)?;
    }
    r.read(m, 0x50b18c, 2)?;
    let n = u32::from(m.u16(0x50b18c)?);
    require(n <= 750, "LoadBatimentGhost count", 0x47bce1)?;
    for i in 0..n {
        m.set_u8(0x50b18c + 4504 + i, 255)?;
    }
    for i in 0..n {
        r.read(m, 0x50b18c + 2 + 2 * i, 2)?;
        r.read(m, 0x50b18c + 1504 + 4 * i, 4)?;
    }
    for i in 0..400 {
        fill(m, 0x50c614 + 10 * i, 8, 255)?;
    }
    let n = u32::from(r.word()?);
    let size = usize::from(r.word()?);
    for i in 0..n {
        require(
            i as usize * 10 + size <= 4000,
            "LoadCadreInfluence bound",
            0x47c0b1,
        )?;
        r.read(m, 0x50c614 + 10 * i, size)?;
    }
    for i in 0..10 {
        m.set_u8(0x4e1d20 + 10 * i, 255)?;
    }
    let n = u32::from(r.word()?);
    let size = usize::from(r.word()?);
    require(n <= 10 && size == 10, "LoadFinMission header", 0x458b05)?;
    for i in 0..n {
        r.read(m, 0x4e1d20 + 10 * i, size)?;
        m.set_u8(0x4e1d88 + i, 0)?;
    }
    r.take(16)?;
    fill(m, 0x511ffc, 800, 255)?;
    for i in 0..400 {
        r.read(m, 0x511ffc + 2 * i, 2)?;
        r.take(1)?;
    }
    monster_init(m, ini)?;
    r.read(m, 0x50a9ce, 2)?;
    let n = u32::from(m.u16(0x50a9ce)?);
    let size = usize::from(r.word()?);
    for i in 0..n {
        require(
            i < 100 && i as usize * 12 + size <= 1200,
            "LoadEntrerMonstre bound",
            0x47ac9b,
        )?;
        let a = 0x50a9d0 + 12 * i;
        r.read(m, a, size)?;
        m.set_u16(a + 4, m.u16(a + 4)?.wrapping_mul(30))?;
        m.set_u32(a + 8, 0)?;
        let mask = m.u16(a + 2)?;
        let mut bit = m.u8(a + 6)?;
        let mut chosen = 255;
        for _ in 0..16 {
            bit = bit.wrapping_add(1) & 15;
            if mask >> bit & 1 != 0 {
                chosen = bit;
                break;
            }
        }
        m.set_u8(a + 6, chosen)?;
    }
    for i in (0..n).rev() {
        let a = 0x50a9d0 + 12 * i;
        if m.u16(a)? != 65535 {
            let mut empty = None;
            for j in (0..i).rev() {
                if m.u16(0x50a9d0 + 12 * j)? == 65535 {
                    empty = Some(j);
                    break;
                }
            }
            let Some(j) = empty else {
                break;
            };
            m.set_u16(0x50a9d0 + 12 * j, m.u16(a)?)?;
        }
        m.set_u16(0x50a9ce, m.u16(0x50a9ce)?.wrapping_sub(1))?;
    }
    Ok(())
}
