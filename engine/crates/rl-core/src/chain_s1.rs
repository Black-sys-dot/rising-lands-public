//! Core S1 query/list provider effects from immutable consultas_luta/consultas_estado_core.
use crate::{
    frame::stop,
    player::{structure, unit, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
pub fn entry(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    match name {
        "ChercheBatimentProximiteCaveOuSanctuaire" => {
            let c = r.eax & 65535;
            let p = r.edx & 255;
            let mut best = [(u32::MAX, 65535); 2];
            for s in 0..1000 {
                let a = structure(s);
                let t = m.b(a + 8)?;
                if m.b(a + 5)? == 0 || m.b(a + 4)? != p || !matches!(t, 2 | 3) {
                    continue;
                }
                let cell = crate::frame_core::building_cell(m, s)? & 65535;
                let dx = (c & 255).abs_diff(cell & 255);
                let dy = (c >> 8).abs_diff(cell >> 8);
                let d = dx * dx + dy * dy;
                let k = (t - 2) as usize;
                if d < best[k].0 {
                    best[k] = (d, s)
                }
            }
            r.eax = if best[1].1 != 65535 {
                best[1].1
            } else {
                best[0].1
            };
            r.edx = 1000;
        }
        "TerrainEstMontagne" => {
            let t = m.w(m.d(0x507d70)? + 2 * (r.eax & 65535))?;
            r.eax = (t & 0xff00) | u32::from((0x1dd..0x260).contains(&t));
        }
        "EnginEstVivant" => {
            if r.eax & 65535 >= 800 {
                return stop("EnginEstVivant unit", 0x443dad);
            }
            r.eax = u32::from(!matches!(m.b(unit(m, r.eax & 65535)? + 2)?, 0 | 4));
        }
        "BatimentGetDecalageConstruction" => {
            r.eax = crate::player_orders::construction_offset(r.eax & 255)
        }
        "GetCaseIndiceBatiment" => {
            if r.eax & 65535 >= 1000 {
                return stop("GetCaseIndiceBatiment index", 0x41aac4);
            }
            r.eax = (((r.eax & 65535) * 69) & 0xffff0000) | m.w(structure(r.eax & 65535) + 2)?;
        }
        "ChercheCaseBatiment" => {
            let c = r.eax & 65535;
            r.eax = 65535;
            for s in 0..1000 {
                let a = structure(s);
                if m.b(a + 5)? == 0 {
                    continue;
                }
                let o = m.w(a + 2)?;
                let t = m.b(a + 8)?;
                let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
                if (0..h).any(|y| (0..w).any(|x| o + (y << 8) + x == c)) {
                    r.eax = s;
                    break;
                }
            }
        }
        "BatimentGetCaseAdjacent" => {
            let s = r.eax & 65535;
            let c = r.edx & 65535;
            if s >= 1000 {
                return stop("BatimentGetCaseAdjacent index", 0x423a6b);
            }
            let (x, y) = ((c & 255) as i32, (c >> 8) as i32);
            let mut result = None;
            let mut edx = u32::MAX;
            'outer: for dy in -1..=1 {
                edx = y as u32;
                if !(0..256).contains(&(y + dy)) {
                    continue;
                }
                for dx in -1..=1 {
                    edx = x as u32;
                    if !(0..256).contains(&(x + dx)) {
                        continue;
                    }
                    let a = structure(s);
                    let t = m.b(a + 8)?;
                    let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
                    let o = m.w(a + 2)?;
                    edx = t * 52;
                    let candidate = (c as i32 + dy * 256 + dx) as u32 & 65535;
                    for cy in 0..h {
                        for cx in 0..w {
                            edx = o + (cy << 8) + cx;
                            if edx != candidate {
                                continue;
                            }
                            let sub = (w * cy + cx) & 65535;
                            edx = if t <= 21 { t } else { sub };
                            if crate::frame_core::call(m, "BatimentCollision", &[t, sub])? & 255
                                == 0
                            {
                                continue;
                            }
                            if r.esp == 0 {
                                return stop("BatimentGetCaseAdjacent stack residue", 0x423997);
                            }
                            let base = r.esp.wrapping_sub(0x40);
                            let row = (m.w(base.wrapping_add(0x26))? << 16) | (dy as u32 & 65535);
                            let col = (m.w(base.wrapping_add(0x1e))? << 16) | (dx as u32 & 65535);
                            result = Some((row << 8).wrapping_add(col).wrapping_add(r.edx));
                            break 'outer;
                        }
                    }
                }
            }
            r.eax = result.unwrap_or(65535);
            r.edx = edx;
        }
        "listDeleteElem" => {
            let a = r.eax;
            let e = r.edx;
            let ptr = m.d(a + 8)?.wrapping_mul(e).wrapping_add(m.d(a + 12)?);
            m.sd(ptr, m.d(a)?)?;
            m.sd(a, e)?;
            m.sd(a + 4, m.d(a + 4)?.wrapping_add(1))?;
            r.eax = e;
        }
        "listUnlinkElem" => {
            let a = r.eax;
            let target = r.edx;
            let (size, base) = (m.d(a + 8)?, m.d(a + 12)?);
            let ptr = base.wrapping_add(size.wrapping_mul(target));
            let (mut prev, mut current) = (u32::MAX, m.d(a)?);
            let mut visited = std::collections::BTreeSet::new();
            while current != u32::MAX {
                if !visited.insert(current) {
                    return stop("listUnlinkElem cyclic list", 0x47d11a);
                }
                if current == target {
                    let next = m.d(ptr)?;
                    m.sd(
                        if prev == u32::MAX {
                            a
                        } else {
                            base.wrapping_add(size.wrapping_mul(prev))
                        },
                        next,
                    )?;
                    m.sd(a + 4, m.d(a + 4)?.wrapping_sub(1))?;
                    break;
                }
                prev = current;
                current = m.d(base.wrapping_add(size.wrapping_mul(current)))?;
            }
            r.eax = current;
            r.edx = current;
        }
        "iaProbReturnTo" => {
            let index = r.eax;
            let a = 0x512320u32.wrapping_add(96u32.wrapping_mul(index));
            m.sb(a + 0x5e, 0)?;
            m.sb(a + 0x5f, 1)?;
            m.sd(a + 0x14, 0)?;
            m.sd(a + 0x54, r.edx)?;
            r.eax = a;
            r.edx = index;
        }
        "VoixOffAsk" => {
            let s = r.eax & 65535;
            if s >= 1000 {
                return stop("VoixOffAsk structure", 0x41a1b5);
            }
            let channel = structure(s) + 0x100;
            crate::player::sound(
                m,
                0x41a1d3,
                false,
                [channel, r.edx, 0, (r.ecx & !255) | (r.ebx & 255)],
                None,
            )?;
            let (mode, b) = match r.edx & 65535 {
                0 => (0, 0),
                1 => (1, 3),
                2 => (2, 0),
                3 => (1, 0),
                4 => (1, 1),
                5 => (2, 1),
                6 => (2, 2),
                _ => (0, r.edx),
            };
            let edx = ((mode & 65535) << 16) | ((b & 65535) << 8);
            r.eax = crate::player::sound(
                m,
                0x41a221,
                true,
                [0x41, edx, channel, r.ebx & 255],
                Some(0x100),
            )?;
            r.ebx = channel;
            r.edx = edx;
        }
        _ => return stop("S1 Core provider", 0x45315f),
    }
    Ok(r)
}
