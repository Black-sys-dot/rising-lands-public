//! iagWarStartBuildUnits 0x487089, reached from the iaSolver RULE
//! I_CommandeForceDefense 0x47ff34 (player [+0x1c], wanted strength [+0]).
//!
//! Three static unit tables of (unit kind, building kind) pairs ending at 0xff
//! (A 0x498cec, B 0x498ce0, C 0x498cd8) are scanned with
//! TypeEnginEstRealisable and iagVilleGetNbBatBuiltType 0x484b76; each entry
//! writes the player's flag byte 0x51ce24 + 0x1144 * player + kind. With no
//! flagged entry the result is 0xff. Otherwise, while fewer than three passes
//! ran and the accumulated strength is below the wanted one, pass i rescans
//! table A, B or C, draws iaRandom(count) to pick the r-th flagged entry, keeps
//! the first picked kind as the result and, unless a problem 3 for that kind
//! already exists, tries the AI unit cheat in the first non-destroyed building
//! of the kind's building type (EnginNewPossible, BagAccepte(1), CreerEngin of
//! a kind-9 unit at the building, BatimentLogeEngin, EnginSetEtat(14), head of
//! the player's kind-10 group list), then posts problem 3 for the kind and adds
//! the kind's strength from 0x498cf4.
use crate::{frame::stop, MemoriaJogo, Result, WorldRng};

const TABLES: [u32; 3] = [0x498cec, 0x498ce0, 0x498cd8];

fn flags(player: u32) -> u32 {
    0x51ce24u32.wrapping_add(player.wrapping_mul(0x1144))
}
/// One table pass (0x4870af..0x4870f8): returns the flagged-entry count.
fn scan(m: &mut MemoriaJogo, player: u32, table: u32) -> Result<u32> {
    let base = flags(player);
    let mut entry = table;
    let mut count = 0u32;
    for _ in 0..256 {
        let kind = m.u8(entry)?;
        if kind == 0xff {
            return Ok(count);
        }
        // TypeEnginEstRealisable 0x44228e (AL = player) and
        // iagVilleGetNbBatBuiltType 0x484b76 (WORD, tested as a signed dword).
        let p = player & 0xff;
        let realisable = m.u8(0x4e1934 + 26 * p + u32::from(kind))? != 0;
        let built = realisable && {
            let row = player.wrapping_mul(124);
            let building = u32::from(m.u8(entry + 1)?);
            m.u16(0x51b84cu32.wrapping_add(row).wrapping_add(2 * building))? != 0
        };
        m.set_u8(base.wrapping_add(u32::from(kind)), u8::from(built))?;
        count += u32::from(built);
        entry = entry.wrapping_add(2);
    }
    stop("iagWarStartBuildUnits unterminated unit table", 0x4870af)
}
fn ai_random(m: &mut MemoriaJogo, n: u32) -> Result<u32> {
    let mut rng = WorldRng {
        seed: m.u32(0x498db4)?,
    };
    let value = rng.draw() % n;
    m.set_u32(0x498db4, rng.seed)?;
    Ok(value)
}
/// GetTypeBatimentFromTypeUnite 0x41cb9a for the kinds of the three tables.
fn building_type(kind: u8) -> Option<u8> {
    Some(match kind {
        0..=2 | 8 => 1,
        3 | 25 => 13,
        11 | 12 | 15 => 5,
        _ => return None,
    })
}
/// 0x48734c..0x4873eb: the unit cheat in the first usable building.
fn create(m: &mut MemoriaJogo, player: u32, kind: u8) -> Result<()> {
    let Some(building_kind) = building_type(kind) else {
        // Every kind of the static tables is associated; another kind would
        // read the caller's DL.
        return stop("iagWarStartBuildUnits unassociated unit kind", 0x487344);
    };
    let mut start = 0u32;
    let building = loop {
        let index = crate::frame_spawn::call(
            m,
            "GetNumIndiceTypeBatiment",
            &[start, u32::from(building_kind), player & 0xff],
        )? & 0xffff;
        if index == 0xffff {
            return Ok(());
        }
        // BatimentEnDestruction 0x41a70b.
        if m.u8(crate::player::structure(index) + 5)? != 3 {
            break index;
        }
        start = index + 1;
    };
    let r = crate::player::structure(building);
    // EnginNewPossible 0x457e0c: a free unit slot exists.
    let pool = m.u32(0x4e19a0)?;
    let mut free = false;
    for i in 0..800 {
        if m.u8(pool + 276 * i + 2)? == 0 {
            free = true;
            break;
        }
    }
    if !free {
        return Ok(());
    }
    let accepts = crate::chain_bags::body(
        m,
        "BagAccepte",
        crate::FrameRegisters {
            eax: r + 12,
            edx: 1,
            ebx: r,
            ..Default::default()
        },
    )?
    .eax as u8;
    if accepts == 0 {
        return Ok(());
    }
    let newborn = crate::frame_spawn::call(
        m,
        "CreerEngin",
        &[u32::from(m.u16(r + 2)?), 1, 9, u32::from(m.u8(r + 4)?)],
    )? as u16;
    crate::chain_entry::lodge(m, building as u16, newborn)?;
    let u = crate::player::unit(m, u32::from(newborn))?;
    if !matches!(m.u8(u + 2)?, 0 | 4) {
        m.set_u8(u + 2, 14)?;
        crate::frame_spawn::release(m, u32::from(newborn))?;
    }
    // iagUnitGetFirstGroupFromType 0x483867 (player, 10).
    let group = m.u32(
        0x51b664u32
            .wrapping_add(player.wrapping_mul(96))
            .wrapping_add(40),
    )?;
    crate::frame_raid::call(m, "iagUnitAddToGroup", &[group, u32::from(newborn)])?;
    Ok(())
}
/// Returns the native EAX (0xff when nothing is buildable or picked).
pub fn start_build_units(m: &mut MemoriaJogo, player: u32, wanted: u32) -> Result<u32> {
    let mut total = 0u32;
    for table in TABLES {
        total = total.wrapping_add(scan(m, player, table)?);
    }
    if total == 0 {
        return Ok(0xff);
    }
    let base = flags(player);
    let mut result = 0xffu32;
    let mut strength = 0i32;
    let mut pass = 0;
    while pass < 3 && strength < wanted as i32 {
        let table = TABLES[pass];
        pass += 1;
        let count = scan(m, player, table)?;
        if (count as i32) <= 0 {
            continue;
        }
        let pick = ai_random(m, count)?;
        let mut entry = table;
        let mut guard = 0;
        while m.u8(base.wrapping_add(u32::from(m.u8(entry)?)))? == 0 {
            entry = entry.wrapping_add(2);
            guard += 1;
            if guard > 256 {
                return stop("iagWarStartBuildUnits flag scan", 0x4872ae);
            }
        }
        let mut seen = 0;
        while seen < pick {
            entry = entry.wrapping_add(2);
            if m.u8(base.wrapping_add(u32::from(m.u8(entry)?)))? != 0 {
                seen += 1;
            }
            guard += 1;
            if guard > 256 {
                return stop("iagWarStartBuildUnits flag scan", 0x4872d4);
            }
        }
        let kind = m.u8(entry)?;
        if result == 0xff {
            result = u32::from(kind);
        }
        if kind == 0xff {
            continue;
        }
        if crate::ai_kernels::search(m, player, 3, &[u32::from(kind)])? != u32::MAX {
            continue;
        }
        create(m, player, kind)?;
        crate::frame_tasks::post(m, &[player, 3, u32::from(kind), 0, 0, 0, 0, 0])?;
        strength = strength.wrapping_add(m.u32(0x498cf4 + 4 * u32::from(kind))? as i32);
    }
    Ok(result)
}
