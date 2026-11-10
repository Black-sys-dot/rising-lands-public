//! Natural remapping and MissionPlaceTerrain's ordered physical insertion.
use crate::{
    frame_core, frame_spawn,
    mission_load::{fill, require},
    MemoriaJogo, Result,
};
pub fn remap(m: &mut MemoriaJogo) -> Result<bool> {
    m.set_u8(0x4e0f36, 0)?;
    m.set_u8(0x4e0f37, 0)?;
    m.set_u32(0x4e0f3c, 0)?;
    m.set_u16(0x496304, 65535)?;
    fill(m, m.u32(0x4e156c)?, 65536, 0)?;
    fill(m, 0x4e1670, 256, 1)?;
    m.set_u8(0x4e1570, 255)?;
    for i in 1..256 {
        m.set_u8(0x4e1570 + i, i as u8)?;
    }
    let regions = m.u32(0x4e0f38)?;
    fill(m, regions, 131072, 0)?;
    let attr = m.u32(0x507da8)?;
    let tiles = m.u32(0x507d70)?;
    let objects = m.u32(0x507d74)?;
    let markers = m.u32(0x507db0)?;
    let mut classes = vec![0u8; 65536];
    for c in 0..65536u32 {
        let tile = u32::from(m.u16(tiles + 2 * c)?);
        let o = m.u8(objects + c)?;
        let obstacle = (tile < 702 && m.u8(markers + tile)? != 0) || (1..=11).contains(&o);
        classes[c as usize] = if obstacle { m.u8(attr + c)? & 1 } else { 0 };
    }
    let mut labels = vec![0u16; 65536];
    let mut count = 0u16;
    let mut stack = Vec::new();
    for start in 0..65536usize {
        if labels[start] != 0 {
            continue;
        }
        count = count.wrapping_add(1);
        let class = classes[start];
        stack.push(start);
        while let Some(c) = stack.pop() {
            if labels[c] != 0 || classes[c] != class {
                continue;
            }
            labels[c] = count;
            let (x, y) = (c & 255, c >> 8);
            if x > 0 {
                stack.push(c - 1);
            }
            if x < 255 {
                stack.push(c + 1);
            }
            if y < 255 {
                stack.push(c + 256);
            }
            if y > 0 {
                stack.push(c - 256);
            }
            if class == 0 {
                if x > 0 && y > 0 {
                    stack.push(c - 257);
                }
                if x > 0 && y < 255 {
                    stack.push(c + 255);
                }
                if x < 255 && y > 0 {
                    stack.push(c - 255);
                }
                if x < 255 && y < 255 {
                    stack.push(c + 257);
                }
            }
        }
    }
    let (w, h) = (usize::from(m.u16(0x497168)?), usize::from(m.u16(0x49716a)?));
    require(
        w <= 256 && h <= 256,
        "Chemin_Remap map dimensions",
        0x42f0e2,
    )?;
    for y in 0..256 {
        for x in if y < h { w } else { 0 }..256 {
            let c = y * 256 + x;
            labels[c] = 0;
            m.set_u8(attr + c as u32, 1)?;
        }
    }
    let bytes: Vec<_> = labels.into_iter().flat_map(u16::to_le_bytes).collect();
    m.write(regions, &bytes)?;
    Ok(count < 0x4000)
}
fn place_unit(m: &mut MemoriaJogo, i: u32) -> Result<()> {
    let u = frame_spawn::unit(m, i)?;
    let c = m.u16(u + 8)?;
    frame_core::set_unit_collision(m, i, u32::from(c))?;
    m.set_u8(u + 0x72, 1)?;
    let owner = m.u8(u)?;
    if !matches!(owner, 4 | 200) {
        let radius = u32::from(m.u8(u + 0xbb)?) + u32::from(m.u8(u + 0xf5)?);
        crate::player_sight::area(m, c, radius + 2, owner, 0)?;
        crate::player_sight::area(m, c, radius, owner, 1)?;
    }
    crate::player_sight::minimap(m, owner, c, 4)?;
    Ok(())
}
pub fn place(m: &mut MemoriaJogo) -> Result<()> {
    for i in 0..800 {
        let u = frame_spawn::unit(m, i)?;
        if m.u8(u + 2)? != 0 && m.u8(u + 0x72)? != 0 {
            place_unit(m, i)?;
        }
    }
    for p in 0..4 {
        for i in 0..800 {
            let u = frame_spawn::unit(m, i)?;
            if matches!(m.u8(u + 2)?, 0 | 4) || u32::from(m.u8(u)?) != p {
                continue;
            }
            let t = u32::from(m.u8(u + 1)?);
            let base = match t {
                5 | 6 | 7 | 11 | 14 | 16 => 1,
                12 => 2,
                13 => 5,
                15 => 3,
                17 => 6,
                _ => 0,
            };
            let bonus = crate::frame_economy::bonus(m, p, t, 1, 0)? as u8;
            m.set_u16(u + 12, (base as u16).wrapping_add(u16::from(bonus)))?;
        }
    }
    for i in 0..1000 {
        let a = 0x49a0c8 + 276 * i;
        if m.u8(a + 5)? == 0 {
            continue;
        }
        crate::player_build::place(m, i)?;
        if crate::mission_tail::inhabited(m, i)? {
            crate::mission_tail::habitable(m, i)?;
        }
        let t = u32::from(m.u8(a + 8)?);
        let owner = m.u8(a + 4)?;
        let origin = u32::from(m.u16(a + 2)?);
        let w = u32::from(m.u16(0x495d8c + 52 * t)?);
        let h = u32::from(m.u16(0x495d8c + 52 * t + 2)?);
        for y in 0..h {
            for x in 0..w {
                if (origin & 255) + x < 256 && (origin >> 8) + y < 256 {
                    crate::player_sight::minimap(m, owner, (origin + y * 256 + x) as u16, 6)?;
                }
            }
        }
    }
    Ok(())
}
