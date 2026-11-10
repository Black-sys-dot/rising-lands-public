//! Native terrain-only RestoreAll effects and physical scene geometry.
//!
//! RestoreAll 0x474b08 calls AffTerrain twice, never AffSprite. The two terrain
//! calls have no gameplay writes in this domain. Their pixels, clipping and
//! terrain-cache payloads belong to presentation and are NOT returned here.
//! The finite load helper retains its historical domain; production uses geometry.
use crate::{EngineError, MemoriaJogo, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadTerrainDomain {
    pub map: (u16, u16),
    pub origin: (u16, u16),
    pub last_terrain_cell: u16,
    pub sprite_margin_fits: bool,
}
fn require(ok: bool, name: &'static str, address: u32) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(EngineError::UnportedCallee { name, address })
    }
}
/// Only the eight actual MissionLoad origins measured by stage_display_* are
/// admitted. This is not a general camera/all-stage render acceptance predicate.
/// Camera 0, 800x600, 25x17, margin 0 and the 256-slot cache are explicit inputs.
/// No unit/building/projectile body is dispatched by this terrain-only boundary.
pub fn load_terrain_domain(m: &MemoriaJogo) -> Result<LoadTerrainDomain> {
    let stage = m.u8(0x496fe4)?;
    let map = (m.u16(0x497168)?, m.u16(0x49716a)?);
    let origin = (m.u16(0x508720)?, m.u16(0x508722)?);
    let expected = match stage {
        1 => ((64, 64), (4, 6)),
        2 => ((64, 64), (0, 47)),
        6 => ((256, 128), (2, 31)),
        7 => ((192, 192), (75, 49)),
        8 => ((128, 128), (0, 45)),
        17 => ((256, 256), (215, 116)),
        23 => ((256, 256), (231, 138)),
        25 => ((192, 192), (161, 4)),
        _ => {
            return Err(EngineError::UnportedCallee {
                name: "Stage display unsampled load profile",
                address: 0x474b08,
            })
        }
    };
    require(
        (map, origin) == expected,
        "Stage display unsampled map/origin",
        0x41068e,
    )?;
    terrain_domain(m, map, origin)
}
/// Shared terrain geometry, independent of the historical load-only profiles.
/// The native terrain traversal depends on the viewport and physical backing,
/// not the mission number or a list of previously observed camera origins.
/// Sprite bodies remain checked by the renderer at each actual dispatch.
pub fn scene_terrain_domain(m: &MemoriaJogo) -> Result<LoadTerrainDomain> {
    let map = (m.u16(0x497168)?, m.u16(0x49716a)?);
    let origin = (m.u16(0x508720)?, m.u16(0x508722)?);
    require(
        map.0 <= 256 && map.1 <= 256,
        "Scene physical map dimensions",
        0x41068e,
    )?;
    terrain_domain(m, map, origin)
}
fn terrain_domain(
    m: &MemoriaJogo,
    map: (u16, u16),
    origin: (u16, u16),
) -> Result<LoadTerrainDomain> {
    require(
        m.u8(0x508041)? == 0,
        "Stage display unsampled camera",
        0x41068e,
    )?;
    require(
        (m.u16(0x508724)?, m.u16(0x508726)?) == (25, 17)
            && (m.u16(0x49a018)?, m.u16(0x49a01a)?) == (800, 600)
            && m.u32(0x4e17a0)? == 800
            && (0..=8).contains(&(m.u8(0x508f90)? as i8)),
        "Stage display unsampled viewport/margin",
        0x41068e,
    )?;
    require(
        m.u16(0x495bc4)? == 256 && m.u16(0x49a04e)? < 256,
        "Stage display unsampled cache",
        0x40fd95,
    )?;
    let last = (u32::from(origin.1) + 16) * 256 + u32::from(origin.0) + 24;
    require(
        u32::from(origin.0) + 25 <= u32::from(map.0)
            && u32::from(origin.1) + 17 <= u32::from(map.1)
            && last <= 65535,
        "Stage display terrain sample bounds",
        0x41068e,
    )?;
    // Require the actual backing allocations. No replacement bytes are supplied.
    for (slot, len) in [
        (0x507d70, 131072),
        (0x507d74, 65536),
        (0x507dac, 65536),
        (0x507d78, 65536),
        (0x507d88, 65536),
    ] {
        m.read(m.u32(slot)?, len)?;
    }
    Ok(LoadTerrainDomain {
        map,
        origin,
        last_terrain_cell: last as u16,
        sprite_margin_fits: origin.0 + 33 <= 256 && origin.1 + 27 <= 256,
    })
}
/// Exact gameplay effects of the two terrain/flip pairs in the finite load
/// domain. Read failures propagate. This does not render or perform host I/O.
/// AffTerrain has no gameplay writes; DDFlipPage 0x43f15c..0x43f1a6 writes the
/// wrapping permutation cursor first, then decrements shake only on even frames.
/// The current frame_input loop already implements this same flip body. Its
/// owner can reuse that loop after separating RestoreAll's domain from AffGame.
pub fn load_terrain_flip_effects(m: &mut MemoriaJogo) -> Result<LoadTerrainDomain> {
    let domain = load_terrain_domain(m)?;
    terrain_flip_effects(m, domain)
}
/// RestoreAll's production terrain-only effects, admitted by physical geometry.
/// Mission identity does not participate in AffTerrain or its two flip calls.
/// The finite historical helper above retains its original evidence boundary.
pub fn restore_terrain_flip_effects(m: &mut MemoriaJogo) -> Result<LoadTerrainDomain> {
    let domain = scene_terrain_domain(m)?;
    terrain_flip_effects(m, domain)
}
fn terrain_flip_effects(
    m: &mut MemoriaJogo,
    domain: LoadTerrainDomain,
) -> Result<LoadTerrainDomain> {
    for _ in 0..2 {
        let shake = m.u8(0x4dfb44)?;
        if shake != 0 {
            // Native also reads this byte to compute a presentation offset.
            // It does not alter the seed/permutation; preserve the read contract.
            let cursor = m.u8(0x4e1d18)?;
            m.u8(0x4e1c18 + u32::from(cursor))?;
            m.set_u8(0x4e1d18, cursor.wrapping_add(1))?;
            if m.u8(0x508f7c)? & 1 == 0 {
                m.set_u8(0x4dfb44, shake - 1)?;
            }
        }
    }
    Ok(domain)
}

/// AffSprite's first (ground-object/building) pass is 33 columns by 22 rows
/// for a 25x17 viewport. 0x416a88/0x416b19/0x416ac6/0x416b5a maintain a packed
/// linear index and truncate it to u16 for reads. Crossing x=255 advances into
/// the following backing row; this is not independent x/y clamping or modulo.
/// This helper returns only that sample address, not a rendered-body acceptance.
pub fn sprite_object_cell(origin: (u16, u16), col: u16, row: u16) -> Result<u16> {
    require(
        origin.0 <= 255 && origin.1 <= 255 && col < 33 && row < 22,
        "Stage display sprite object sample bounds",
        0x416b55,
    )?;
    Ok(origin
        .1
        .wrapping_mul(256)
        .wrapping_add(origin.0)
        .wrapping_add(row.wrapping_mul(256))
        .wrapping_add(col))
}

/// Building records actually reached by AffSprite416864's ground pass and
/// AffBatiments407dd3's ownership/discovery prefix. The admitted viewport has
/// 33x22 packed-u16 samples. This selects preflight checks, not supported bodies;
/// visible facades, walls and gates retain their addressed renderer boundaries.
pub fn scene_building_indices(m: &MemoriaJogo) -> Result<[bool; 1000]> {
    let domain = scene_terrain_domain(m)?;
    let grid = m.u32(0x4e0f38)?;
    let discovery = m.u32(0x507d78)?; // scene_terrain_domain admits camera0.
    let terrain = m.u32(0x507d70)?;
    let objects = m.u32(0x507d74)?;
    let mut selected = [false; 1000];
    for row in 0..22 {
        for col in 0..33 {
            let cell = u32::from(sprite_object_cell(domain.origin, col, row)?);
            let region = m.u16(grid.wrapping_add(2 * cell))?;
            if region & 0x8000 == 0 {
                continue;
            }
            // AffBatiments0x40f23b dispatches per cell: only the wall/gate
            // object codes and the nineteen facade terrain words reach a body.
            // Any other word (MONDE23's 0x418, intact0x40f342) draws nothing
            // and writes nothing, so that cell admits no body effects.
            let code = m.u8(objects.wrapping_add(cell))?;
            let tile = u32::from(m.u16(terrain.wrapping_add(2 * cell))?);
            if !matches!(code, 253 | 254) && !crate::render_scene::FACADES.contains(&tile) {
                continue;
            }
            let index = usize::from((region & 0x7fe0) >> 5);
            require(index < 1000, "Scene structure index beyond pool", 0x407e1e)?;
            let r = 0x49a0c8 + 276 * index as u32;
            if m.u8(r + 5)? != 3 && m.u8(r + 4)? != 0 && m.u8(r + 0x110)? == 0 {
                continue;
            }
            if m.u8(discovery.wrapping_add(cell))? & 16 != 0 {
                continue;
            }
            selected[index] = true;
        }
    }
    Ok(selected)
}
