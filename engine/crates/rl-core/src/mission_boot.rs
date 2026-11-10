//! Asset-based gameplay startup. No captured MissionLoad entry or S0 is required.
//! The PE loader supplies original initialized data and zero-filled .bss; all heap
//! bytes use an explicit allocation-fill input, matching the native test boundary.
use crate::{
    mission_ini::MissionIni,
    mission_load::{bag_reset, fill, require},
    mission_startup, CrtRng, FrameRegisters, MemoriaJogo, Result, WorldRng,
};
use std::collections::BTreeMap;

pub struct StartupAssets<'a> {
    pub executable: &'a [u8],
    pub rising_ini: &'a [u8],
    pub rules: &'a [u8],
    pub symbols: &'a [u8],
    pub obstacles: &'a [u8],
    pub terrain_colors: &'a [u8],
    pub palette_pie: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct StartupInput {
    pub allocation_fill: u8,
    pub ai_clock_ms: u32,
    pub thread_id: u32,
}
fn bytes(b: &[u8], o: usize, n: usize) -> Result<&[u8]> {
    b.get(o..o.saturating_add(n))
        .ok_or(crate::EngineError::IncompleteField { address: o as u32 })
}
fn word(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes(b, o, 2)?.try_into().expect("u16")))
}
fn dword(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes(b, o, 4)?.try_into().expect("u32")))
}
fn cstring(b: &[u8], o: usize) -> Result<&[u8]> {
    let tail = b
        .get(o..)
        .ok_or(crate::EngineError::IncompleteField { address: o as u32 })?;
    let n = tail
        .iter()
        .position(|c| *c == 0)
        .ok_or(crate::EngineError::IncompleteField { address: o as u32 })?;
    Ok(&tail[..n])
}
/// Load original PE sections at their original addresses; uninitialized bytes are OS zero fill.
pub fn image(executable: &[u8]) -> Result<MemoriaJogo> {
    require(
        bytes(executable, 0, 2)? == b"MZ",
        "startup MZ image",
        0x400000,
    )?;
    let pe = dword(executable, 60)? as usize;
    require(
        bytes(executable, pe, 4)? == b"PE\0\0" && word(executable, pe + 4)? == 0x14c,
        "startup PE image",
        0x400000,
    )?;
    let mut image = vec![0u8; 0x200000];
    let count = word(executable, pe + 6)?;
    let opt = usize::from(word(executable, pe + 20)?);
    require(
        dword(executable, pe + 24 + 28)? == 0x400000,
        "startup PE base",
        0x400000,
    )?;
    for i in 0..usize::from(count) {
        let h = pe + 24 + opt + i * 40;
        let rva = dword(executable, h + 12)? as usize;
        let n = dword(executable, h + 16)? as usize;
        let source = dword(executable, h + 20)? as usize;
        if n == 0 || source == 0 {
            continue;
        }
        let dest = image
            .get_mut(rva..rva.saturating_add(n))
            .ok_or(crate::EngineError::AddressOverflow)?;
        dest.copy_from_slice(bytes(executable, source, n)?);
    }
    MemoriaJogo::from_blocks([(0x400000, image)])
}
struct Arena {
    next: u32,
    fill: u8,
}
impl Arena {
    fn alloc(&mut self, m: &mut MemoriaJogo, n: usize) -> Result<u32> {
        let p = self.next;
        self.next = self
            .next
            .checked_add((n as u32 + 4095) & !4095)
            .ok_or(crate::EngineError::AddressOverflow)?;
        m.declare(p, vec![self.fill; n])?;
        Ok(p)
    }
    fn slot(&mut self, m: &mut MemoriaJogo, slot: u32, n: usize) -> Result<u32> {
        let p = self.alloc(m, n)?;
        m.set_u32(slot, p)?;
        Ok(p)
    }
}
/// iaProbLoadBinaryRules 0x47e437 and debugSymLoad/Find's SBL relocation.
/// Uses the shipped binary rule and symbol assets, resolving every native function address.
pub fn ai_rules(m: &mut MemoriaJogo, rules: &[u8], symbols: &[u8]) -> Result<()> {
    require(
        bytes(rules, 0, 4)? == b"RULE",
        "iaProbLoadBinaryRules signature",
        0x47e47b,
    )?;
    let n = usize::from(word(rules, 12)?);
    let nr = usize::from(word(rules, 14)?);
    let strings = dword(rules, 8)? as usize;
    require(
        (4..=1200).contains(&n) && nr <= 64,
        "iaProbLoadBinaryRules table size",
        0x47e485,
    )?;
    m.write(0x514910, bytes(rules, 16, n * 20)?)?;
    m.write(0x51a720, bytes(rules, 16 + n * 20, nr * 2)?)?;
    let strings = bytes(rules, 16 + n * 20 + nr * 2, strings)?;
    let decoded: Vec<_> = symbols.iter().map(|b| !*b).collect();
    let mut names = BTreeMap::new();
    let mut cursor = 0;
    loop {
        let a = dword(&decoded, cursor)?;
        if a == u32::MAX {
            break;
        }
        let name = cstring(&decoded, dword(&decoded, cursor + 4)? as usize)?.to_vec();
        names.insert(name, a);
        cursor += 8;
    }
    let base = 0x48ff7fu32.wrapping_sub(
        *names
            .get(b"_debugGetPC ".as_slice())
            .ok_or(crate::EngineError::IncompleteField { address: 0x48d74b })?,
    );
    m.set_u32(0x514900, n as u32 - 4)?;
    for i in 4..n {
        let a = 0x514910 + i as u32 * 20;
        let raw = m.u32(a)?;
        if raw & 0x80000000 == 0 {
            continue;
        }
        let name = cstring(strings, (raw & 0x7fffffff) as usize)?;
        if name == b"NULL" {
            m.set_u32(a, 0)?;
            continue;
        }
        let mut key = name.to_vec();
        key.push(b' ');
        let address = *names.get(&key).ok_or(crate::EngineError::UnportedCallee {
            name: "iaProbLoadBinaryRules unresolved SBL symbol",
            address: 0x47e5fb,
        })?;
        m.set_u32(a, base.wrapping_add(address))?;
    }
    Ok(())
}
fn structures(m: &mut MemoriaJogo) -> Result<()> {
    fill(m, 0x49a070, 88, 0)?;
    for i in 0..1000 {
        let a = 0x49a0c8 + 276 * i;
        for (o, v) in [
            (5, 0),
            (0x90, 1),
            (0, 255),
            (0x74, 0),
            (0x75, 0),
            (0x76, 0),
            (0x7b, 255),
            (0x7a, 0),
            (0x104, 0),
            (0x8a, 1),
            (0x8b, 1),
            (4, 255),
            (8, 255),
        ] {
            m.set_u8(a + o, v)?;
        }
        for (o, v) in [
            (10, 0),
            (0x78, 65535),
            (0x86, 65535),
            (0x88, 65535),
            (6, 0),
            (0x82, 0),
            (0x7c, 0),
            (0x80, 0),
            (0x7e, 0),
            (0xc4, 0),
            (0xc6, 0),
            (0xc2, 0),
            (0x84, 200),
            (0xc8, 65535),
            (2, 65535),
        ] {
            m.set_u16(a + o, v)?;
        }
        for (o, v) in [(0x100, u32::MAX), (0x8c, 0), (0x108, 0), (0x110, 0)] {
            m.set_u32(a + o, v)?;
        }
    }
    for p in 0..4 {
        for t in 0..22 {
            bag_reset(m, 0x4dd6e8 + 2288 * p + 104 * t)?;
        }
        m.set_u16(0x4dfb2c + 6 * p, 0)?;
        m.set_u16(0x4dfb2e + 6 * p, 100)?;
        m.set_u8(0x4dfb30 + 6 * p, 0)?;
        for a in [0x4dfb48, 0x4dfb4c, 0x4dfb54, 0x4dfb58] {
            m.set_u8(a + p, 0)?;
        }
        m.set_u16(0x4dfb5c + 2 * p, 0)?;
    }
    Ok(())
}
fn init_terrain(m: &mut MemoriaJogo) -> Result<()> {
    let mut rng = WorldRng {
        seed: m.u32(0x497164)?,
    };
    let mut aux = vec![0u8; 65536];
    for v in &mut aux {
        *v = (rng.draw() & 3) as u8;
    }
    m.write(m.u32(0x507dac)?, &aux)?;
    m.set_u32(0x497164, rng.seed)?;
    for slot in [0x507d74, 0x507da8] {
        fill(m, m.u32(slot)?, 65536, 0)?;
    }
    for p in 0..4 {
        for (slot, value) in [(0x507d98, 0), (0x507d88, 31), (0x507d78, 31)] {
            fill(m, m.u32(slot + 4 * p)?, 65536, value)?;
        }
    }
    for i in 0..20 {
        m.set_u16(0x509954 + 4 * i, 0)?;
    }
    Ok(())
}
pub(crate) fn players(m: &mut MemoriaJogo) -> Result<()> {
    let current = m.u32(0x5080a8)?;
    let saved = m.u32(0x5080ac)?;
    m.set_u8(0x508040, 1)?;
    m.set_u8(0x508041, 0)?;
    for p in 0..4 {
        m.set_u8(current + 20 * p, p as u8)?;
        m.set_u8(current + 20 * p + 1, if p == 0 { 1 } else { 2 })?;
        m.set_u8(current + 20 * p + 6, 0)?;
        m.set_u8(saved + 20 * p + 6, 1 << p)?;
        for k in 0..4 {
            m.set_u32(0x5080b0 + 16 * p + 4 * k, 0)?;
            m.set_u16(current + 20 * p + 12 + 2 * k, 0)?;
            m.set_u16(saved + 20 * p + 12 + 2 * k, 0)?;
        }
    }
    Ok(())
}
fn palette(m: &mut MemoriaJogo, pie: &[u8]) -> Result<()> {
    require(
        bytes(pie, 0, 3)? == b"Pie" && word(pie, 7)? == 256,
        "LoadPalette PIE",
        0x4798e9,
    )?;
    let mut rgb: Vec<u8> = bytes(pie, 9, 768)?
        .iter()
        .map(|b| b.wrapping_shl(2))
        .collect();
    rgb[..3].fill(0);
    m.write(m.u32(0x4e24b0)?, &rgb)?;
    let mut shade = vec![0u8; 256];
    for i in 0..256 {
        shade[i] = match i {
            5 => 223,
            6 => 222,
            7 => 221,
            8..=15 => 167,
            _ => {
                let target = [
                    u32::from(rgb[3 * i]) * 60 / 100,
                    u32::from(rgb[3 * i + 1]) * 60 / 100,
                    u32::from(rgb[3 * i + 2]) * 60 / 100,
                ];
                let mut best = 0;
                let mut dist = u32::MAX;
                for c in 0..256 {
                    let d = (0..3)
                        .map(|k| {
                            let v = i32::from(rgb[3 * c + k]) - target[k] as i32;
                            (v * v) as u32
                        })
                        .sum();
                    if d < dist {
                        dist = d;
                        best = c as u8;
                    }
                }
                best
            }
        };
    }
    m.write(m.u32(0x4e24b4)?, &shade)?;
    // ProgInit 0x474f04..0x474f2e: lighting 60%, black-and-white, negative
    // and grey tables follow CalculOmbrage and precede the player palettes.
    crate::palette_tables::prog_init_tables(m, 60)?;
    for p in 0..5 {
        let ptr = m.u32(0x4e24c8 + 4 * p)?;
        for c in 0..256 {
            m.set_u8(ptr + c, c as u8)?;
        }
    }
    for c in 0..8 {
        for p in 1..=3 {
            m.set_u8(
                m.u32(0x4e24c8 + 4 * p)? + 160 + c,
                m.u8(0x496d4c + 8 * (p - 1) + c)?,
            )?;
        }
        m.set_u8(m.u32(0x4e24d8)? + 160 + c, 47)?;
    }
    for p in 0..5 {
        m.set_u8(0x4e24dc + p, m.u8(m.u32(0x4e24c8 + 4 * p)? + 160)?)?;
    }
    // ProgInit's three SetCyclage calls happen after CalculOmbrage.
    for (i, start, end) in [(0, 3, 3), (1, 5, 7), (2, 8, 15)] {
        m.set_u8(0x4e24e4 + 3 * i, start)?;
        m.set_u8(0x4e24e5 + 3 * i, end)?;
        m.set_u8(0x4e24e6 + 3 * i, 0)?;
    }
    m.set_u8(0x4e24e1, 3)?;
    Ok(())
}
/// Gameplay portion of ProgInit and the new-campaign setup up to MissionLoad.
/// Display/menu resources are the host's responsibility. This initializes every
/// gameplay input consumed by the native MissionLoad composition.
pub fn startup(assets: &StartupAssets<'_>, input: StartupInput) -> Result<MemoriaJogo> {
    startup_inner(assets, input, None)
}
pub fn startup_with_events(
    assets: &StartupAssets<'_>,
    input: StartupInput,
    host: &crate::events_host::Input<'_>,
) -> Result<MemoriaJogo> {
    host.validate()?;
    startup_inner(assets, input, Some(host))
}
fn startup_inner(
    assets: &StartupAssets<'_>,
    input: StartupInput,
    host: Option<&crate::events_host::Input<'_>>,
) -> Result<MemoriaJogo> {
    let mut m = prog_init_inner(assets, input, host)?;
    // New-campaign MenuInit path: stage 1, fresh experience, stored research,
    // then RechercheCalculNew before CalculGame calls MissionLoad. The briefing
    // confirmation 0x42cb02 leaves the original load request mode0.
    m.set_u8(0x4e2f78, 0)?;
    m.set_u8(0x496fe4, 1)?;
    for p in 0..4 {
        for i in 0..5 {
            m.set_u16(0x4e19ec + 10 * p + 2 * i, 255)?;
        }
    }
    let mut technology = crate::chain_technology::Technology::default();
    technology.entry(&mut m, "RechercheCalculNew", FrameRegisters::default())?;
    prepare_load_frame(&mut m)?;
    Ok(m)
}

/// Gameplay portion of ProgInit 0x474beb: allocations, INI tables, rules, AI
/// tables, palette and the intro screen mode1 it returns with. The original
/// first CalculGame frame then enters the main menu; nothing campaign-specific
/// is prepared here. Display/menu resources remain the host's responsibility.
pub fn prog_init(assets: &StartupAssets<'_>, input: StartupInput) -> Result<MemoriaJogo> {
    prog_init_inner(assets, input, None)
}
pub fn prog_init_with_events(
    assets: &StartupAssets<'_>,
    input: StartupInput,
    host: &crate::events_host::Input<'_>,
) -> Result<MemoriaJogo> {
    host.validate()?;
    prog_init_inner(assets, input, Some(host))
}
fn prog_init_inner(
    assets: &StartupAssets<'_>,
    input: StartupInput,
    host: Option<&crate::events_host::Input<'_>>,
) -> Result<MemoriaJogo> {
    let mut m = image(assets.executable)?;
    let ini = MissionIni::parse(assets.rising_ini);
    let mut arena = Arena {
        next: 0x40000000,
        fill: input.allocation_fill,
    };
    m.declare(0x3400000, vec![0; 0xe4])?;
    m.set_u32(0x522244, 0x3400000)?;
    m.set_u32(0x340000c, 1)?;
    m.set_u32(0x34000da, input.thread_id)?;
    m.set_u32(0x497164, 2)?; // ProgInit Randomize(1): seed = argument + 1.
    let mut crt = CrtRng { seed: 1 };
    let mut free = [true; 256];
    for i in 0..256 {
        let mut value = crt.draw() as u8;
        while !free[usize::from(value)] {
            value = value.wrapping_add(1);
        }
        free[usize::from(value)] = false;
        m.set_u8(0x4e1c18 + i, value)?;
    }
    m.set_u32(0x340000c, crt.seed)?;
    m.set_u8(0x4e1d18, 0)?;
    // ProgInit474c07 permutation precedes EVENTS474d53; later types/load follow.
    if let Some(host) = host {
        host.initialize(&mut m)?;
    }
    mission_startup::spell_types(&mut m, &ini)?;
    for (slot, n) in [
        (0x4e24b0, 768),
        (0x4e24b4, 256),
        (0x4e24b8, 256),
        (0x4e24bc, 256),
        (0x4e24c0, 256),
        (0x4e24c4, 256),
        (0x4e24c8, 256),
        (0x4e24cc, 256),
        (0x4e24d0, 256),
        (0x4e24d4, 256),
        (0x4e24d8, 256),
    ] {
        arena.slot(&mut m, slot, n)?;
    }
    let units = arena.slot(&mut m, 0x4e19a0, 220800)?;
    let damage = arena.slot(&mut m, 0x4e199c, 108)?;
    let routes = arena.slot(&mut m, 0x4e1924, 3200)?;
    for i in 0..800 {
        let p = arena.alloc(&mut m, 512)?;
        m.set_u32(routes + 4 * i, p)?;
    }
    for i in 0..27 {
        let p = arena.alloc(&mut m, 27)?;
        m.set_u32(damage + 4 * i, p)?;
    }
    let orders = arena.slot(&mut m, 0x4e19b8, 16)?;
    for p in 0..4 {
        let a = arena.alloc(&mut m, 2704)?;
        m.set_u32(orders + 4 * p, a)?;
        for t in 0..26 {
            bag_reset(&mut m, a + 104 * t)?;
        }
    }
    mission_startup::unit_types(&mut m, &ini)?;
    for i in 0..800 {
        let a = units + 276 * i;
        if m.u8(a + 2)? != 0 {
            m.set_u16(a + 0x9a, 0)?;
            m.set_u8(a + 2, 0)?;
            m.set_u8(a + 0xaf, 0)?;
            bag_reset(&mut m, a + 10)?;
        }
    }
    structures(&mut m)?;
    mission_startup::structure_types(&mut m, &ini)?;
    for (slot, n) in [
        (0x507d74, 65536),
        (0x507d70, 131072),
        (0x507da8, 65536),
        (0x507dac, 65536),
        (0x507db0, 702),
    ] {
        arena.slot(&mut m, slot, n)?;
    }
    for p in 0..4 {
        for slot in [0x507d98, 0x507d88, 0x507d78] {
            arena.slot(&mut m, slot + 4 * p, 65536)?;
        }
    }
    init_terrain(&mut m)?;
    arena.slot(&mut m, 0x5080a8, 100)?;
    arena.slot(&mut m, 0x5080ac, 100)?;
    let n = usize::from(m.u16(0x496fe0)?) * usize::from(m.u16(0x496fe2)?);
    for (slot, value) in [(0x507d5c, 2), (0x507d60, 0), (0x507d64, 0)] {
        let a = arena.slot(&mut m, slot, n)?;
        fill(&mut m, a, n, value)?;
    }
    arena.slot(&mut m, 0x4e0f38, 131072)?;
    arena.slot(&mut m, 0x4e156c, 65536)?;
    ai_rules(&mut m, assets.rules, assets.symbols)?;
    mission_startup::ai_tables(&mut m, input.ai_clock_ms)?;
    arena.slot(&mut m, 0x5080f0, 5600)?;
    let mut technology = crate::chain_technology::Technology::default();
    for p in 0..4 {
        for o in [0, 1, 2, 3, 4] {
            m.set_u8(0x508134 + 16 * p + o, 0)?;
        }
        for o in [8, 12] {
            m.set_u32(0x508134 + 16 * p + o, 0)?;
        }
        m.set_u8(0x50871c + p, 0)?;
    }
    technology.entry(&mut m, "BonusReset", FrameRegisters::default())?;
    mission_startup::invention_types(&mut m, &ini)?;
    players(&mut m)?;
    require(
        assets.obstacles.len() >= 702,
        "TerrainLoadObstacle asset",
        0x475302,
    )?;
    let markers = m.u32(0x507db0)?;
    m.write(markers, &assets.obstacles[..702])?;
    let n = assets.terrain_colors.len().min(702);
    m.write(0x509694, &assets.terrain_colors[..n])?;
    palette(&mut m, assets.palette_pie)?;
    // ProgInit 0x474f6e..0x474f7e: no pending margin, intro screen mode1.
    m.set_u8(0x508f90, 0)?;
    m.set_u8(0x4e2f78, 1)?;
    Ok(m)
}

/// CalculGame 0x42d1da..0x42d228 gameplay prelude: InitGouttes draws in slot order.
/// This runs on each mission-load frame, after startup/menu research setup.
pub fn prepare_load_frame(m: &mut MemoriaJogo) -> Result<()> {
    for i in 0..120 {
        let a = 0x4e1da0 + 6 * i;
        let x = crate::frame_core::random(m)? % 510;
        m.set_u16(a, x as u16)?;
        let y = crate::frame_core::random(m)? % 446;
        m.set_u16(a + 2, y as u16)?;
        let phase = crate::frame_core::random(m)? & 3;
        m.set_u8(a + 4, phase as u8)?;
    }
    m.set_u8(0x496d2c, 1)?;
    m.set_u8(0x497285, 1)?;
    m.set_u8(0x496fd8, 0)?;
    m.set_u32(0x4e061c, 0)?;
    Ok(())
}
