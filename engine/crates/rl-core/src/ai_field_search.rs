//! I_ChercheChampNonExploite (0x481426), including its native ring order.
use crate::{EngineError, MemoriaJogo, Result};

fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}

// iagRessTestChampExploite 0x485b72: signed count, full-width player.
fn exploited(m: &MemoriaJogo, player: u32, cell: u32) -> Result<bool> {
    let base = 0x51ba94u32.wrapping_add(player.wrapping_mul(168));
    for i in 0..(m.u32(0x51bcf0)? as i32).max(0) as u32 {
        if m.u32(base.wrapping_add(i.wrapping_mul(8)))? == cell {
            return Ok(true);
        }
    }
    Ok(false)
}

// GetNbrPatatesRayon 0x489034. Discarding coordinates outside the physical
// 256-square map is equivalent to the native bounds checks, without reads.
fn potatoes(m: &MemoriaJogo, cell: u32, radius: i32) -> Result<u32> {
    let x = (cell & 255) as i32;
    let y = (cell >> 8) as i32;
    let bounds = |v: i32| -> Result<(i32, i32)> {
        let lo = v.checked_sub(radius);
        let hi = v.checked_add(radius);
        match (lo, hi) {
            (Some(lo), Some(hi)) if hi != i32::MAX => Ok((lo.max(0), hi.min(255))),
            _ => Err(stop("GetNbrPatatesRayon overflowing loop bounds", 0x48905c)),
        }
    };
    let (left, right) = bounds(x)?;
    let (top, bottom) = bounds(y)?;
    let terrain = m.u32(0x507d74)?;
    let mut sum = 0u32;
    for x in left..=right {
        for y in top..=bottom {
            let value = m.u8(terrain.wrapping_add((y * 256 + x) as u32))?;
            if (29..=52).contains(&value) {
                sum = sum.wrapping_add(u32::from(value - 28));
            }
        }
    }
    Ok(sum)
}

pub fn search(m: &mut MemoriaJogo, player: u32) -> Result<u32> {
    let mut city_start = 0;
    loop {
        let city = crate::frame_spawn::call(
            m,
            "GetNumIndiceTypeBatiment",
            &[city_start, 2, player & 255],
        )? & 65535;
        if city == 65535 {
            return Ok(u32::MAX);
        }
        let origin = crate::frame_spawn::call(m, "GetCaseIndiceBatiment", &[city])? & 65535;
        let mut caves = Vec::new();
        let mut distances = Vec::new();
        let mut cave_start = 0;
        loop {
            let cave = crate::frame_spawn::call(
                m,
                "GetNumIndiceTypeBatiment",
                &[cave_start, 3, player & 255],
            )? & 65535;
            if cave == 65535 {
                break;
            }
            let cell = crate::frame_spawn::call(m, "GetCaseIndiceBatiment", &[cave])? & 65535;
            if crate::frame_spawn::accessible(m, origin as u16, cell as u16)? {
                if caves.len() == 100 {
                    return Err(stop(
                        "I_ChercheChampNonExploite native local array overflow",
                        0x4814e3,
                    ));
                }
                caves.push(cell);
                let dx = (origin & 255) as i32 - (cell & 255) as i32;
                let dy = (origin >> 8) as i32 - (cell >> 8) as i32;
                distances.push((dx * dx + dy * dy) as u16);
            }
            cave_start = cave + 1;
        }
        for _ in 0..caves.len() {
            // The original chooses the last i smaller than ANY other j.
            // This is deliberately not a nearest-cave sort, including ties.
            let mut best = 0;
            for i in 0..caves.len() {
                for j in 0..caves.len() {
                    if i != j && distances[i] < distances[j] {
                        best = i;
                    }
                }
            }
            distances[best] = 65535;
            let cave = caves[best];
            let x = (cave & 255) as i32;
            let y = (cave >> 8) as i32;
            // The first ring executes even when the configured limit is <=1.
            // Radii >=256 cannot intersect this map for a WORD origin.
            let limit = (m.u32(0x51bcdc)? as i32).clamp(2, 256);
            for radius in 1..limit {
                for axis in 0..2 {
                    for offset in -radius..=radius {
                        for edge in [-radius, radius] {
                            let (cx, cy) = if axis == 0 {
                                (x + offset, y + edge)
                            } else {
                                (x + edge, y + offset)
                            };
                            if !(0..256).contains(&cx) || !(0..256).contains(&cy) {
                                continue;
                            }
                            let cell = (cy * 256 + cx) as u32;
                            if m.u8(m.u32(0x507d74)?.wrapping_add(cell))? == 28
                                && crate::frame_spawn::accessible(m, cave as u16, cell as u16)?
                                && !exploited(m, player, cell)?
                                && potatoes(m, cell, m.u32(0x51bce8)? as i32)? > m.u32(0x51bcec)?
                            {
                                return Ok(cell);
                            }
                        }
                    }
                }
            }
        }
        city_start = city + 1;
    }
}

/// I_DonneCaveAssocieeChamp481738 / Mine48179e: low-word cell and low-byte
/// owner. Both originals compare full squared distances; the last tie wins.
pub fn nearest_cave(m: &mut MemoriaJogo, cell: u32, player: u32) -> Result<u32> {
    let cell = cell & 65535;
    let mut start = 0;
    let mut best = u32::MAX;
    let mut least = u32::MAX;
    loop {
        let cave =
            crate::frame_spawn::call(m, "GetNumIndiceTypeBatiment", &[start, 3, player & 255])?
                & 65535;
        if cave == 65535 {
            return Ok(best);
        }
        let other = crate::frame_spawn::call(m, "GetCaseIndiceBatiment", &[cave])? & 65535;
        let dx = (cell & 255) as i32 - (other & 255) as i32;
        let dy = (cell >> 8) as i32 - (other >> 8) as i32;
        let distance = (dx * dx + dy * dy) as u32;
        if distance <= least {
            least = distance;
            best = cave;
        }
        start = cave + 1;
    }
}

/// I_DonneValeurChamp4817f2 passes the full cell DWORD and square radius.
/// Native489034 ignores terrain outside its physical256-square backing.
pub fn field_value(m: &MemoriaJogo, cell: u32) -> Result<u32> {
    potatoes(m, cell, m.u32(0x51bce8)? as i32)
}

/// I_DonneMaxExploitantsDansChamp481806: signed IDIV of the full value DWORD.
pub fn max_workers(m: &MemoriaJogo, value: u32) -> Result<u32> {
    (value as i32)
        .checked_div(m.u32(0x51bcec)? as i32)
        .map(|v| v as u32)
        .ok_or_else(|| {
            stop(
                "I_DonneMaxExploitantsDansChamp x86 signed division fault",
                0x481815,
            )
        })
}

/// V_SetChampExploite485d7b: idempotent insertion in the first free slot.
pub fn set_exploited(m: &mut MemoriaJogo, cell: u32, group: u32, player: u32) -> Result<u32> {
    if exploited(m, player, cell)? {
        return Ok(1);
    }
    let offset = player.wrapping_mul(168);
    let base = 0x51ba94u32.wrapping_add(offset);
    for i in 0..(m.u32(0x51bcf0)? as i32).max(0) as u32 {
        let slot = base.wrapping_add(i.wrapping_mul(8));
        if m.u32(slot)? == u32::MAX {
            m.set_u32(slot, cell)?;
            // Native group association precedes the field DWORD by four bytes.
            m.set_u32(slot.wrapping_sub(4), group)?;
            let counter = 0x51bac4u32.wrapping_add(offset);
            m.set_u32(counter, m.u32(counter)?.wrapping_add(1))?;
            break;
        }
    }
    Ok(1)
}

/// V_SetChampNonExploite485dfc: the first slot holding `cell` is freed (field and
/// group DWORDs to -1, the player's counter 0x51bac4 decremented); always 1.
pub fn clear_exploited(m: &mut MemoriaJogo, cell: u32, player: u32) -> Result<u32> {
    let offset = player.wrapping_mul(168);
    let base = 0x51ba94u32.wrapping_add(offset);
    for i in 0..(m.u32(0x51bcf0)? as i32).max(0) as u32 {
        let slot = base.wrapping_add(i.wrapping_mul(8));
        if m.u32(slot)? == cell {
            m.set_u32(slot, u32::MAX)?;
            m.set_u32(slot.wrapping_sub(4), u32::MAX)?;
            let counter = 0x51bac4u32.wrapping_add(offset);
            m.set_u32(counter, m.u32(counter)?.wrapping_sub(1))?;
            break;
        }
    }
    Ok(1)
}

// GetIndiceMineFromCaseFreePath4890fa accepts only the lower two entrances.
// Its index is only a predicate here; MineGetIndice can pick an earlier overlap.
fn mine_entrance(m: &MemoriaJogo, cell: u32) -> Result<bool> {
    for i in 0..20 {
        let origin = u32::from(m.u16(0x509954 + 4 * i)?);
        if origin != 0 && (cell == origin + 256 || cell == origin + 257) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// I_ChercheMine481833: each owner CAVE in record order, then native ring order.
pub fn search_mine(m: &mut MemoriaJogo, player: u32) -> Result<u32> {
    let mut start = 0;
    loop {
        let cave =
            crate::frame_spawn::call(m, "GetNumIndiceTypeBatiment", &[start, 3, player & 255])?
                & 65535;
        if cave == 65535 {
            return Ok(u32::MAX);
        }
        let origin = crate::frame_spawn::call(m, "GetCaseIndiceBatiment", &[cave])? & 65535;
        let x = (origin & 255) as i32;
        let y = (origin >> 8) as i32;
        // Radius1 always executes. Larger than255 cannot intersect this map;
        // skipping those empty rings preserves the result and gameplay/RNG effects.
        let limit = (m.u32(0x51bce0)? as i32).clamp(2, 256);
        for radius in 1..limit {
            for axis in 0..2 {
                for offset in -radius..=radius {
                    for edge in [-radius, radius] {
                        let (cx, cy) = if axis == 0 {
                            (x + offset, y + edge)
                        } else {
                            (x + edge, y + offset)
                        };
                        if !(0..256).contains(&cx) || !(0..256).contains(&cy) {
                            continue;
                        }
                        let cell = (cy * 256 + cx) as u32;
                        if !mine_entrance(m, cell)?
                            || !crate::frame_spawn::accessible(m, origin as u16, cell as u16)?
                        {
                            continue;
                        }
                        let index = crate::terrain_mine::mine_index(m, cell)?;
                        if index != 65535
                            && m.u8(0x51a9cc + index)? == 0
                            && crate::terrain_mine::mine_capacity(m, index)? != 0
                        {
                            return Ok(u32::from(m.u16(0x509954 + 4 * index)?));
                        }
                    }
                }
            }
        }
        start = cave + 1;
    }
}

/// I_DonneBatimentProche4808b4 additionally requires a shared accessible zone.
/// Type/owner are bytes, the input cell is a WORD, and the last distance tie wins.
pub fn nearest_building(m: &mut MemoriaJogo, cell: u32, kind: u32, player: u32) -> Result<u32> {
    let cell = cell as u16;
    let mut start = 0;
    let mut best = u32::MAX;
    let mut least = u32::MAX;
    loop {
        let index = crate::frame_spawn::call(
            m,
            "GetNumIndiceTypeBatiment",
            &[start, kind & 255, player & 255],
        )? & 65535;
        if index == 65535 {
            return Ok(best);
        }
        let target = crate::frame_spawn::call(m, "GetCaseIndiceBatiment", &[index])? as u16;
        if crate::frame_spawn::accessible(m, cell, target)? {
            let target = crate::frame_spawn::call(m, "GetCaseIndiceBatiment", &[index])? & 65535;
            let dx = i32::from(cell & 255) - (target & 255) as i32;
            let dy = i32::from(cell >> 8) - (target >> 8) as i32;
            let distance = (dx * dx + dy * dy) as u32;
            if distance <= least {
                least = distance;
                best = index;
            }
        }
        start = index + 1;
    }
}
