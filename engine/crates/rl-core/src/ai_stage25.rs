//! Measured MONDE25 RULE callers and their shared group continuations.
//! Original-width operations and reached callees; unknown dependencies stay explicit.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

/// GetCaseOfGhostFromCaseCentrale 0x47bc6b, including BatimentTailleGet
/// 0x41a996. Only DL selects the size table, but the central cell is a DWORD.
/// Subtraction wraps over 32 bits, without clamping or truncating to a cell.
pub fn ghost_origin(m: &MemoriaJogo, center: u32, kind: u8) -> Result<u32> {
    let size = 0x495d8cu32.wrapping_add(52 * u32::from(kind));
    let width = u32::from(m.u16(size)? >> 1);
    let height = u32::from(m.u16(size.wrapping_add(2))? >> 1);
    Ok(center.wrapping_sub((height << 8).wrapping_add(width)))
}

/// ImplantePlayerInGhost 0x47b984. Scan the WORD count in native order,
/// skipping ffff slots. Existing owners do not prevent overwriting. The first
/// match wins; owner is AL. No gameplay capacity bound is added to the table.
/// Return the native EAX (only AL is Boolean) and final EDX scan index.
pub fn implant(m: &mut MemoriaJogo, player: u32, origin: u32) -> Result<(u32, u32)> {
    let mut index = 0u32;
    let mut offset = 0u32;
    while index < u32::from(m.u16(0x50b18c)?) {
        let cell = u32::from(m.u16(0x50b18eu32.wrapping_add(offset))?);
        if cell != 0xffff && cell == origin {
            m.set_u8(0x50c324u32.wrapping_add(index), player as u8)?;
            return Ok(((offset & !0xff) | 1, index));
        }
        offset = offset.wrapping_add(2);
        index = index.wrapping_add(1);
    }
    Ok((offset & !0xff, index))
}

/// Typed RULE API: two resolved arguments and the actual scheduler player.
/// Return 1 on a match, ffffffff otherwise, including a wrapped origin miss.
pub fn reserve(m: &mut MemoriaJogo, center: u32, kind: u32, player: u32) -> Result<u32> {
    let origin = ghost_origin(m, center, kind as u8)?;
    let (value, _) = implant(m, player, origin)?;
    Ok(if value as u8 == 1 { 1 } else { u32::MAX })
}

pub fn from_arguments(m: &mut MemoriaJogo, arguments: &[u32], player: u32) -> Result<u32> {
    let [center, kind, ..] = arguments else {
        return stop("B_ReserveGhostBatiment missing arguments", 0x480816);
    };
    reserve(m, *center, *kind, player)
}

/// Full original pointer ABI. Read kind/center before the size lookup and read
/// the player DWORD at +1c afterwards, exactly as the original wrapper does.
/// Every non-EAX GPR is preserved; the plain RET pops four bytes.
pub fn entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let kind = m.u32(r.eax.wrapping_add(4))?;
    let center = m.u32(r.eax)?;
    let origin = ghost_origin(m, center, kind as u8)?;
    let player = m.u32(r.eax.wrapping_add(0x1c))?;
    let (value, _) = implant(m, player, origin)?;
    r.eax = if value as u8 == 1 { 1 } else { u32::MAX };
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Exact leaf register adapters for differential validation.
pub fn leaf_entry(
    m: &mut MemoriaJogo,
    name: &str,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    match name {
        "GetCaseOfGhostFromCaseCentrale" => {
            let width = m.u16(0x495d8cu32.wrapping_add(52 * u32::from(r.edx as u8)))?;
            r.eax = ghost_origin(m, r.eax, r.edx as u8)?;
            r.edx = u32::from(width >> 1);
        }
        "ImplantePlayerInGhost" => (r.eax, r.edx) = implant(m, r.eax, r.edx)?,
        _ => return stop("unknown stage25 leaf", 0x480816),
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// I_DonneBatimentOccupe 0x480760. The owner-only scan includes inactive
/// records; the last qualifying record of each kind wins. The original
/// 0x498588 priority table then selects the first available kind. Occupancy is
/// BagContient(tag=1,type=9), not Bag.count or the building state.
pub fn occupied(m: &mut MemoriaJogo, minimum: u32, player: u32) -> Result<u32> {
    let mut by_kind = [u16::MAX; 22];
    for index in 0..1000u32 {
        let record = 0x49a0c8 + 276 * index;
        if m.u8(record + 4)? != player as u8 {
            continue;
        }
        let count = crate::chain_bags::body(
            m,
            "BagContient",
            FrameRegisters {
                eax: record + 12,
                edx: 1,
                ebx: 9,
                ..Default::default()
            },
        )?
        .eax as u16;
        if i32::from(count) < minimum as i32 {
            continue;
        }
        let kind = usize::from(m.u8(record + 8)?);
        let Some(slot) = by_kind.get_mut(kind) else {
            return stop("I_DonneBatimentOccupe native stack type", 0x4807b9);
        };
        *slot = index as u16;
    }
    for offset in 0..m.u32(0x498588)? {
        let kind = m.u32(0x49858cu32.wrapping_add(offset.wrapping_mul(4)))?;
        let Some(&index) = by_kind.get(kind as usize) else {
            return stop("I_DonneBatimentOccupe native stack priority", 0x4807e0);
        };
        if index != u16::MAX {
            return Ok(u32::from(index));
        }
    }
    Ok(u32::MAX)
}

pub fn occupied_from_arguments(m: &mut MemoriaJogo, arguments: &[u32], player: u32) -> Result<u32> {
    let Some(&minimum) = arguments.first() else {
        return stop("I_DonneBatimentOccupe missing argument", 0x480760);
    };
    occupied(m, minimum, player)
}

/// Pointer ABI preserves every non-EAX register and reads only the low owner
/// byte at +1c, like 480783. Callee-local stack scratch is not world state.
pub fn occupied_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let player = u32::from(m.u8(r.eax.wrapping_add(0x1c))?);
    let minimum = m.u32(r.eax)?;
    r.eax = occupied(m, minimum, player)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// V_BatimentSortColon480712: call the existing original withdrawal body,
/// then normalize its low-word ffff result to the RULE failure dword.
pub fn sort_colon(m: &mut MemoriaJogo, structure: u32) -> Result<u32> {
    let result = crate::chain_recruit::retire_colonist(m, structure & 65535)? as u16;
    Ok(if result == u16::MAX {
        u32::MAX
    } else {
        u32::from(result)
    })
}

pub fn sort_colon_from_arguments(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let Some(&structure) = arguments.first() else {
        return stop("V_BatimentSortColon missing argument", 0x480712);
    };
    sort_colon(m, structure)
}

pub fn sort_colon_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let structure = u32::from(m.u16(r.eax)?);
    r.eax = sort_colon(m, structure)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Message42,482ae5..482b5b: remove the released unit from its owner's
/// housed list, then satisfy a type-specific pending group or use the fallback.
pub(crate) fn released_colonist(m: &mut MemoriaJogo, unit: u16) -> Result<()> {
    let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(unit));
    let owner = u32::from(m.u8(r)?);
    let housed = m.u32(0x51b68c + 96 * owner)?;
    crate::frame_raid::remove_unit(m, housed, unit)?;
    let owner = u32::from(m.u8(r)?);
    let fallback = m.u32(0x51b690 + 96 * owner)?;
    assign_released(m, unit, fallback)
}

/// iagUnitAffecteGroupe4838d1. Prepend to the first waiting group of the
/// requested unit type, wake its task, unlink that waiting node, then clear its
/// requested type. Otherwise prepend to the supplied fallback. No RNG.
pub(crate) fn assign_released(m: &mut MemoriaJogo, unit: u16, fallback: u32) -> Result<()> {
    let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(unit));
    let kind = m.u8(r + 1)?;
    let owner = u32::from(m.u8(r)?);
    let head = 0x51b6b4 + 96 * owner;
    let mut current = m.u32(head)?;
    let mut previous = None;
    let mut traversed = 0;
    while current != u32::MAX {
        let g = 0x51a9e0u32.wrapping_add(current.wrapping_shl(5));
        if m.u8(g.wrapping_add(0x1a))? == kind {
            crate::frame_raid::add_unit(m, current, unit)?;
            let task = m.u32(g.wrapping_add(8))?;
            if task != u32::MAX {
                crate::frame_tasks::wake(m, task, 1)?;
            }
            let link = previous.map_or(head, |p: u32| 0x51a9e4u32.wrapping_add(p.wrapping_shl(5)));
            m.set_u32(link, m.u32(g.wrapping_add(4))?)?;
            m.set_u8(g.wrapping_add(0x1a), 255)?;
            return Ok(());
        }
        previous = Some(current);
        current = m.u32(g.wrapping_add(4))?;
        traversed += 1;
        if traversed > 10000 {
            return stop("iagUnitAffecteGroupe nonprogress waiting list", 0x483919);
        }
    }
    crate::frame_raid::add_unit(m, fallback, unit)?;
    Ok(())
}

/// V_AttendreDispatcher481fc4. Transfer an available unit first; otherwise
/// iagUnitSetWait483ad0 appends this group and sleeps its task. The RULE returns1.
pub fn wait_dispatcher(
    m: &mut MemoriaJogo,
    arguments: &[u32],
    player: u32,
    task: u32,
) -> Result<u32> {
    let [destination, kind, ..] = arguments else {
        return stop("V_AttendreDispatcher missing arguments", 0x481fc4);
    };
    if crate::ai_rules::transfer_unit(m, player, *destination, *kind)? as u16 != u16::MAX {
        return Ok(1);
    }
    let g = 0x51a9e0u32.wrapping_add(destination.wrapping_shl(5));
    if g == 0 {
        return Ok(1);
    }
    let head = 0x51b6b4u32.wrapping_add(player.wrapping_mul(96));
    let mut link = head;
    let mut current = m.u32(link)?;
    let mut traversed = 0;
    while current != u32::MAX {
        link = 0x51a9e4u32.wrapping_add(current.wrapping_shl(5));
        current = m.u32(link)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("iagUnitSetWait nonprogress waiting list", 0x483b06);
        }
    }
    m.set_u32(link, *destination)?;
    m.set_u32(g.wrapping_add(4), u32::MAX)?;
    m.set_u8(g.wrapping_add(0x1a), *kind as u8)?;
    m.set_u32(g.wrapping_add(8), task)?;
    if task != u32::MAX {
        crate::frame_tasks::sleep(m, task, 0)?;
    }
    Ok(1)
}

/// I_DonneGroupeNonComplet482000 / iagUnitFindGroupNotFull483890.
/// Return the first group whose unsigned WORD count is below its WORD limit.
pub fn not_full(m: &MemoriaJogo, arguments: &[u32], player: u32) -> Result<u32> {
    let Some(&kind) = arguments.first() else {
        return stop("I_DonneGroupeNonComplet missing argument", 0x482000);
    };
    let head = 0x51b664u32
        .wrapping_add(player.wrapping_mul(96))
        .wrapping_add(kind.wrapping_mul(4));
    let mut current = m.u32(head)?;
    let mut traversed = 0;
    while current != u32::MAX {
        let g = 0x51a9e0u32.wrapping_add(current.wrapping_shl(5));
        if g == 0 {
            return Ok(u32::MAX);
        }
        if m.u16(g.wrapping_add(0x12))? < m.u16(g.wrapping_add(0x14))? {
            return Ok(current);
        }
        current = m.u32(g)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("iagUnitFindGroupNotFull nonprogress list", 0x4838a7);
        }
    }
    Ok(current)
}

pub fn group_rule_entry(
    m: &mut MemoriaJogo,
    address: u32,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    let player = m.u32(r.eax.wrapping_add(0x1c))?;
    r.eax = if address == 0x481fc4 {
        wait_dispatcher(m, &arguments, player, m.u32(r.eax.wrapping_add(0x20))?)?
    } else if address == 0x482000 {
        not_full(m, &arguments, player)?
    } else {
        return stop("unknown pending group RULE", address);
    };
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// I_DonneForceEnnemieDansZone47fe9b / iagWarComputeEnnemyStrengthInZone487797.
/// Physical tiles select occupants; canonical cells prevent counting a multi-cell
/// object twice. Enemy checks keep their original unit/building alliance direction.
pub fn enemy_zone(m: &mut MemoriaJogo, arguments: &[u32], player: u32) -> Result<u32> {
    let [center, radius, ..] = arguments else {
        return stop("I_DonneForceEnnemieDansZone missing arguments", 0x47fe9b);
    };
    if *center > 65535 || *radius > 255 {
        return stop(
            "iagWarComputeEnnemyStrengthInZone coordinate domain",
            0x487797,
        );
    }
    let (x, y, radius) = (
        (*center & 255) as i32,
        (*center >> 8) as i32,
        *radius as i32,
    );
    let grid = m.u32(0x4e0f38)?;
    let mut strength = 0u32;
    for row in (y - radius).max(0)..=(y + radius).min(255) {
        for column in (x - radius).max(0)..=(x + radius).min(255) {
            let cell = (row * 256 + column) as u32;
            let tile = m.u16(grid.wrapping_add(2 * cell))?;
            if tile & 0xc000 == 0x4000 {
                let index = (tile & 0x3ff8) >> 3;
                let unit = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(index));
                if !matches!(m.u8(unit + 2)?, 0 | 4)
                    && u32::from(m.u16(unit + 0x7c)?) == cell
                    && crate::chain_battle::enemy_unit(m, index, player as u8)?
                {
                    strength =
                        strength.wrapping_add(m.u32(0x498cf4 + 4 * u32::from(m.u8(unit + 1)?))?);
                }
            }
            if tile & 0x8000 != 0 {
                let index = u32::from((tile & 0x7fe0) >> 5);
                let building = 0x49a0c8 + 276 * index;
                if m.u8(building + 5)? != 0
                    && crate::frame_core::call(m, "BatimentGetCase", &[index])? & 65535 == cell
                    && m.u8(building + 5)? != 3
                {
                    let other = u32::from(m.u8(building + 4)?);
                    if other != 200
                        && crate::frame_economy::call(m, "PlayerEstAllie", &[other, player])? as u8
                            == 0
                    {
                        strength = strength
                            .wrapping_add(m.u32(0x498d5c + 4 * u32::from(m.u8(building + 8)?))?);
                    }
                }
            }
        }
    }
    Ok(strength)
}

pub fn enemy_zone_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    let player = m.u32(r.eax.wrapping_add(0x1c))?;
    r.eax = enemy_zone(m, &arguments, player)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// V_ConstruireBatiment481ce3. Iterate the native group cursor, issue each
/// living member's original order, then select group descriptor1. Hidden miners
/// use the existing MineEndWork/PlaceEngin providers and retain their stops.
pub fn construct(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [kind, cell, group, ..] = arguments else {
        return stop("V_ConstruireBatiment missing arguments", 0x481ce3);
    };
    let mut member = crate::frame_raid::first_unit(m, *group)?;
    let mut traversed = 0;
    while member != u16::MAX {
        let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(member));
        let state = m.u8(r + 2)?;
        if !matches!(state, 0 | 4) {
            if state == 5 && m.u8(r + 0x72)? == 0 {
                let mine_cell = u32::from(m.u16(r + 8)?);
                crate::chain_work::leaf(
                    m,
                    "MineEndWork",
                    FrameRegisters {
                        eax: mine_cell,
                        ..Default::default()
                    },
                )?;
                crate::frame_spawn::call(m, "PlaceEngin", &[u32::from(member)])?;
            }
            crate::player_orders::build_order(m, u32::from(member), *kind & 255, *cell & 65535)?;
        }
        member = crate::frame_raid::next_unit(m)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("V_ConstruireBatiment nonprogress group", 0x481cf1);
        }
    }
    let g = 0x51a9e0u32.wrapping_add(group.wrapping_shl(5));
    if g != 0 {
        m.set_u16(g.wrapping_add(0x16), 1)?;
    }
    Ok(1)
}

pub fn construct_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let arguments = [
        m.u32(r.eax)?,
        m.u32(r.eax.wrapping_add(4))?,
        m.u32(r.eax.wrapping_add(8))?,
    ];
    r.eax = construct(m, &arguments)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// V_ReparerBatiment481d7e. Iterate the native group cursor, issue each
/// living member's building repair order, then select group descriptor3. Hidden miners
/// use the existing MineEndWork/PlaceEngin providers and retain their stops.
pub fn repair_group(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [building, group, ..] = arguments else {
        return stop("V_ReparerBatiment missing arguments", 0x481d7e);
    };
    let mut member = crate::frame_raid::first_unit(m, *group)?;
    let mut traversed = 0;
    while member != u16::MAX {
        let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(member));
        let state = m.u8(r + 2)?;
        if !matches!(state, 0 | 4) {
            if state == 5 && m.u8(r + 0x72)? == 0 {
                let mine_cell = u32::from(m.u16(r + 8)?);
                crate::chain_work::leaf(
                    m,
                    "MineEndWork",
                    FrameRegisters {
                        eax: mine_cell,
                        ..Default::default()
                    },
                )?;
                crate::frame_spawn::call(m, "PlaceEngin", &[u32::from(member)])?;
            }
            // SetEnginRepareBatiment444fdb: reset, resolve BatimentGetCase,
            // then select work state15 and the low-word building target.
            crate::frame_spawn::reset_order(m, u32::from(member))?;
            let cell = crate::frame_core::building_cell(m, *building & 65535)? as u16;
            m.set_u16(r + 0x7e, cell)?;
            crate::frame_spawn::release(m, u32::from(member))?;
            m.set_u8(r + 0x88, 0)?;
            m.set_u8(r + 0x89, 0)?;
            m.set_u8(r + 2, 15)?;
            m.set_u16(r + 0xc0, *building as u16)?;
            m.set_u16(r + 0xb2, cell)?;
        }
        member = crate::frame_raid::next_unit(m)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("V_ReparerBatiment nonprogress group", 0x481d8c);
        }
    }
    let g = 0x51a9e0u32.wrapping_add(group.wrapping_shl(5));
    if g != 0 {
        m.set_u16(g.wrapping_add(0x16), 3)?;
    }
    Ok(1)
}

pub fn repair_group_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    r.eax = repair_group(m, &arguments)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// V_ActiveBatiment481e4f. Issue SetEnginRentreBatiment445b10 to every
/// linked member, including dead members; the provider decides state changes.
/// Finally select group descriptor4, without checking building ownership.
pub fn activate_group(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [building, group, ..] = arguments else {
        return stop("V_ActiveBatiment missing arguments", 0x481e4f);
    };
    let mut member = crate::frame_raid::first_unit(m, *group)?;
    let mut traversed = 0;
    while member != u16::MAX {
        crate::chain_entry::set_enter(m, member, *building as u16)?;
        member = crate::frame_raid::next_unit(m)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("V_ActiveBatiment nonprogress group", 0x481e5e);
        }
    }
    let g = 0x51a9e0u32.wrapping_add(group.wrapping_shl(5));
    if g != 0 {
        m.set_u16(g.wrapping_add(0x16), 4)?;
    }
    Ok(1)
}

pub fn activate_group_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    r.eax = activate_group(m, &arguments)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// iagVilleIncBatBuilt484b57: wrapping WORD per-kind and DWORD total counts.
fn increment_built(m: &mut MemoriaJogo, player: u32, kind: u32) -> Result<()> {
    let city = player.wrapping_mul(124);
    let by_kind = 0x51b84cu32
        .wrapping_add(city)
        .wrapping_add(kind.wrapping_mul(2));
    m.set_u16(by_kind, m.u16(by_kind)?.wrapping_add(1))?;
    let total = 0x51b8b8u32.wrapping_add(city);
    m.set_u32(total, m.u32(total)?.wrapping_add(1))
}

/// V_MiseAJourBatimentConstruit482630: retire the construction count first,
/// then publish the completed count. Both indices retain their DWORD width.
pub fn built_update(m: &mut MemoriaJogo, a: &[u32], player: u32) -> Result<u32> {
    let Some(&kind) = a.first() else {
        return stop("V_MiseAJourBatimentConstruit missing type", 0x482630);
    };
    crate::ai_stage25_ghost_chain::decrement(m, player, kind)?;
    increment_built(m, player, kind)?;
    Ok(1)
}

pub fn built_update_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let p = r.eax;
    let kind = m.u32(p)?;
    let player = m.u32(p.wrapping_add(0x1c))?;
    crate::ai_stage25_ghost_chain::decrement(m, player, kind)?;
    // The original wrapper re-reads both arguments after the first mutation.
    let kind = m.u32(p)?;
    let player = m.u32(p.wrapping_add(0x1c))?;
    increment_built(m, player, kind)?;
    r.eax = 1;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// I_AttenteMessageGroupe481f1a / iagMessSetWait4841f4. Store the complete
/// message mask and task, then sleep unconditionally when the group pointer is
/// non-null. Unlike iagUnitSetWait, task ffffffff has no special native branch.
pub fn wait_message(m: &mut MemoriaJogo, arguments: &[u32], task: u32) -> Result<u32> {
    let [group, mask, ..] = arguments else {
        return stop("I_AttenteMessageGroupe missing arguments", 0x481f1a);
    };
    let g = 0x51a9e0u32.wrapping_add(group.wrapping_shl(5));
    if g != 0 {
        m.set_u32(g.wrapping_add(0x1c), *mask)?;
        m.set_u32(g.wrapping_add(8), task)?;
        crate::frame_tasks::sleep(m, task, 0)?;
    }
    Ok(1)
}

pub fn wait_message_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    let task = m.u32(r.eax.wrapping_add(0x20))?;
    r.eax = wait_message(m, &arguments, task)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// V_LibereGhostBatiment480842. LiberePlayerInGhost47ba04 uses the same
/// first-match scan as implantation, writing owner255; the caller returns1 even
/// on a miss. It ignores incoming player and keeps full-width center arithmetic.
pub fn release_ghost(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [center, kind, ..] = arguments else {
        return stop("V_LibereGhostBatiment missing arguments", 0x480842);
    };
    let origin = ghost_origin(m, *center, *kind as u8)?;
    implant(m, 255, origin)?;
    Ok(1)
}

pub fn release_ghost_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let arguments = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    r.eax = release_ghost(m, &arguments)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// I_DonneCaseLibreProcheGhostBatiment480617. ChercheType440f9c selects the
/// first alive kind9 of the low-byte player; EnginCherchePlace4436ab consumes
/// low-word center/index. A missing unit is not replaced with a fabricated one.
pub fn near_ghost(m: &mut MemoriaJogo, arguments: &[u32], player: u32) -> Result<u32> {
    let Some(&center) = arguments.first() else {
        return stop(
            "I_DonneCaseLibreProcheGhostBatiment missing argument",
            0x480617,
        );
    };
    let pool = m.u32(0x4e19a0)?;
    let mut target = u16::MAX;
    for index in 0..800u16 {
        let r = pool.wrapping_add(276 * u32::from(index));
        if !matches!(m.u8(r + 2)?, 0 | 4) && m.u8(r + 1)? == 9 && m.u8(r)? == player as u8 {
            target = index;
            break;
        }
    }
    Ok(u32::from(crate::spell_remaining::find_place(
        m,
        center as u16,
        target,
    )?))
}

pub fn near_ghost_entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let arguments = [u32::from(m.u16(r.eax)?)];
    let player = u32::from(m.u8(r.eax.wrapping_add(0x1c))?);
    r.eax = near_ghost(m, &arguments, player)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// I_AiJeUniteDansGroupe481f2e / iagUnitFindTypeIntoGroup483747.
/// Search the linked members in order; kind is a full DWORD and dead members
/// are not filtered. The RULE widens low-word ffff to ffffffff.
pub fn find_group_kind(m: &MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [group, kind, ..] = arguments else {
        return stop("I_AiJeUniteDansGroupe missing arguments", 0x481f2e);
    };
    let g = 0x51a9e0u32.wrapping_add(group.wrapping_shl(5));
    if g == 0 {
        return Ok(u32::MAX);
    }
    let mut member = m.u16(g.wrapping_add(0x10))?;
    let mut traversed = 0;
    while member != u16::MAX {
        let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(member));
        if u32::from(m.u8(r + 1)?) == *kind {
            return Ok(u32::from(member));
        }
        member = m.u16(r + 0x108)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("iagUnitFindTypeIntoGroup nonprogress list", 0x48375a);
        }
    }
    Ok(u32::MAX)
}

/// V_DeplaceGroupe481f4c. Preserve an alive member's attack target when
/// state1 and target+b8 is not ffff; otherwise issue SetDeplaceEngin4457d3.
/// Unlike the construction/repair callers, this does not change the descriptor.
pub fn move_group(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [group, cell, ..] = arguments else {
        return stop("V_DeplaceGroupe missing arguments", 0x481f4c);
    };
    let mut member = crate::frame_raid::first_unit(m, *group)?;
    let mut traversed = 0;
    while member != u16::MAX {
        let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(member));
        let state = m.u8(r + 2)?;
        if !matches!(state, 0 | 4) && !(state == 1 && m.u16(r + 0xb8)? != u16::MAX) {
            crate::frame_spawn::order(m, u32::from(member), *cell as u16)?;
        }
        member = crate::frame_raid::next_unit(m)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("V_DeplaceGroupe nonprogress group", 0x481f58);
        }
    }
    Ok(1)
}

pub fn member_rule_entry(
    m: &mut MemoriaJogo,
    address: u32,
    mut r: FrameRegisters,
) -> Result<FrameRegisters> {
    let args = [m.u32(r.eax)?, m.u32(r.eax.wrapping_add(4))?];
    r.eax = match address {
        0x481f2e => find_group_kind(m, &args)?,
        0x481f4c => move_group(m, &args)?,
        _ => return stop("unknown group member RULE", address),
    };
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_calls_never_supply_a_center_type_or_player() {
        let mut m = MemoriaJogo::from_blocks([(0x1200000, vec![0; 8])]).unwrap();
        let before = m.clone();
        for arguments in [&[][..], &[0][..]] {
            assert!(matches!(
                from_arguments(&mut m, arguments, 0),
                Err(crate::EngineError::UnportedCallee {
                    address: 0x480816,
                    ..
                })
            ));
        }
        assert!(entry(
            &mut m,
            FrameRegisters {
                eax: 0x1200000,
                ..FrameRegisters::default()
            }
        )
        .is_err());
        assert_eq!(m, before);
    }
}
