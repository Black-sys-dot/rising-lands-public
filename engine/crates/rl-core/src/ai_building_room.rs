//! I_DonneBatimentPasPlein 0x480951 (iaSolver RULE; player at [+0x1c]).
//!
//! For each structure of the player (GetNumIndiceBatimentPlayer 0x41aa54 over
//! the 1000 records, owner only) whose bag accepts one more unit
//! (DetectPlaceLibreIndBatiment 0x48927f = BagAccepte(bag, 1)), the routine
//! keeps per structure type the structure with the fewest foot soldiers
//! (GetNbTroupesAPiedsBatiment 0x488e2c: unit slots whose kind is a troop for
//! TypeEnginEstTroupe 0x442347; strictly fewer replaces). Then the types listed
//! in the dword table 0x4985cc (count first) are tried in order: the first type
//! with a kept structure gives the result, -1 otherwise. The native per-type
//! arrays are 22 words on the stack; a structure or list type of 22 or more
//! would write/read the caller's frame and is an explicit stop.
use crate::{frame::stop, MemoriaJogo, Result};

fn troops(m: &MemoriaJogo, building: u32) -> Result<u32> {
    let bag = crate::player::structure(building) + 12;
    let count = u32::from(m.u16(bag + 2)?);
    let pool = m.u32(0x4e19a0)?;
    let mut n = 0;
    for i in 0..count {
        if m.u8(bag + 0x42 + i)? != 1 {
            continue;
        }
        let u = u32::from(m.u16(bag + 6 + 2 * i)?);
        let kind = m.u8(pool.wrapping_add(u * 276 + 1))?;
        if !matches!(kind, 3 | 11..=17 | 25) {
            n += 1;
        }
    }
    Ok(n)
}

pub fn not_full(m: &mut MemoriaJogo, player: u32) -> Result<u32> {
    let mut best = [0xffffu16; 22];
    let mut fewest = [0xffffu16; 22];
    let owner = player as u8;
    let mut start = 0u32;
    loop {
        let mut found = None;
        let mut i = start;
        while (i & 0xffff) < 1000 {
            if m.u8(crate::player::structure(i) + 4)? == owner {
                found = Some(i & 0xffff);
                break;
            }
            i = i.wrapping_add(1);
        }
        let Some(building) = found else { break };
        let r = crate::player::structure(building);
        let accepts = crate::chain_bags::body(
            m,
            "BagAccepte",
            crate::FrameRegisters {
                eax: r + 12,
                edx: 1,
                ebx: 1,
                ..Default::default()
            },
        )?
        .eax as u8;
        if accepts == 1 {
            let n = troops(m, building)?;
            let kind = usize::from(m.u8(r + 8)?);
            if kind >= 22 {
                return stop(
                    "I_DonneBatimentPasPlein structure type outside the stack arrays",
                    0x4809bc,
                );
            }
            if u32::from(fewest[kind]) > n {
                best[kind] = building as u16;
                fewest[kind] = n as u16;
            }
        }
        start = building + 1;
    }
    let count = m.u32(0x4985cc)?;
    let mut i = 1u32;
    while i <= count {
        let kind = m.u32(0x4985cc + 4 * i)?;
        if kind >= 22 {
            return stop(
                "I_DonneBatimentPasPlein list type outside the stack arrays",
                0x480a0c,
            );
        }
        if best[kind as usize] != 0xffff {
            return Ok(u32::from(best[kind as usize]));
        }
        i += 1;
    }
    Ok(u32::MAX)
}
