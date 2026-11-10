//! MissionLoad 0x46c2cf: reset, ETA readers and original post-load composition.
//! Inputs are file bytes and live startup state, never a captured post-load world.
use crate::{frame_core, frame_spawn, mission_ini::MissionIni, MemoriaJogo, Result};

pub(crate) fn require(ok: bool, name: &'static str, address: u32) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(crate::EngineError::UnportedCallee { name, address })
    }
}
pub(crate) fn fill(m: &mut MemoriaJogo, address: u32, size: usize, value: u8) -> Result<()> {
    m.write(address, &vec![value; size])
}
pub(crate) fn copy(m: &mut MemoriaJogo, dest: u32, source: u32, size: usize) -> Result<()> {
    let b = m.read(source, size)?.to_vec();
    m.write(dest, &b)
}
pub(crate) struct Reader<'a> {
    pub bytes: &'a [u8],
    pub position: usize,
}
impl Reader<'_> {
    pub fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self
            .position
            .checked_add(n)
            .ok_or(crate::EngineError::AddressOverflow)?;
        let b = self
            .bytes
            .get(self.position..end)
            .ok_or(crate::EngineError::UnportedCallee {
                name: "MissionLoad truncated ETA",
                address: 0x458461,
            })?;
        self.position = end;
        Ok(b)
    }
    pub fn word(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("word")))
    }
    pub fn read(&mut self, m: &mut MemoriaJogo, a: u32, n: usize) -> Result<()> {
        m.write(a, self.take(n)?)
    }
}
/// Complete MakeMissionName 0x46c1b8 over live stage/variant/sympathy state.
pub fn mission_name(m: &MemoriaJogo, number: u8) -> Result<Vec<u8>> {
    let mut name = crate::mission_startup::string(m, 0x494b27)?;
    require(name.len() == 14, "MakeMissionName prefix", 0x46c1b8)?;
    name[12] = b'0' + number / 10;
    name[13] = b'0' + number % 10;
    let variant = if number == 9 {
        Some(u32::from(m.u8(0x4e1d85)?))
    } else if matches!(number, 10 | 12 | 17) {
        let players = m.u32(0x5080a8)?;
        let mut threshold = 90i8;
        let mut choice = 1;
        for p in 1..4 {
            let sympathy = m.u16(players + 20 * p + 12)? as u8 as i8;
            if threshold > sympathy {
                threshold = sympathy;
                choice = p;
            }
        }
        Some(choice - 1)
    } else {
        None
    };
    if let Some(v) = variant {
        require(v < 4, "MakeMissionName variant table", 0x46c243)?;
        name.extend(crate::mission_startup::string(m, m.u32(0x497154 + 4 * v)?)?);
    }
    name.extend(crate::mission_startup::string(m, 0x494b36)?);
    Ok(name)
}
pub(crate) fn bag_reset(m: &mut MemoriaJogo, a: u32) -> Result<()> {
    for o in [0, 0x66, 4, 2] {
        m.set_u16(a + o, 0)?;
    }
    for i in 0..30 {
        m.set_u16(a + 6 + 2 * i, 0)?;
        m.set_u8(a + 0x42 + i, 0)?;
    }
    Ok(())
}
/// Original reset prefix. All RNG draws occur even if the subsequent file open fails.
pub fn reset(m: &mut MemoriaJogo) -> Result<()> {
    m.set_u16(0x4e1a38, 65535)?;
    m.set_u16(0x4e1a3a, 65535)?;
    for i in 0..10 {
        let a = 0x4e2410 + 6 * i;
        let b = [0xb1, 0x29, 1, 0, 0, 0];
        if m.read(a, 6)? != b {
            m.set_u8(0x4e05a2, 1)?;
        }
        m.write(a, &b)?;
    }
    let objects = m.u32(0x507d74)?;
    let aux = m.u32(0x507dac)?;
    let attrs = m.u32(0x507da8)?;
    for c in 0..65536 {
        m.set_u8(objects + c, 0)?;
        let r = frame_core::random(m)?;
        m.set_u8(aux + c, (r & 3) as u8)?;
        m.set_u8(attrs + c, 0)?;
    }
    for owner in 0..4 {
        for (slot, value) in [(0x507d98, 0), (0x507d88, 31), (0x507d78, 31)] {
            fill(m, m.u32(slot + owner * 4)?, 65536, value)?;
        }
    }
    for i in 0..20 {
        m.set_u16(0x509954 + 4 * i, 0)?;
    }
    let n = usize::from(m.u16(0x496fe0)?) * usize::from(m.u16(0x496fe2)?);
    for (slot, value) in [(0x507d5c, 2), (0x507d60, 0), (0x507d64, 0)] {
        fill(m, m.u32(slot)?, n, value)?;
    }
    fill(m, 0x495be0, 8, 255)?;
    fill(m, 0x49a070, 88, 0)?;
    for i in 0..1000 {
        let a = 0x49a0c8 + 276 * i;
        for (o, v) in [
            (5, 0),
            (0, 255),
            (0x74, 0),
            (0x75, 0),
            (0x76, 0),
            (0x7b, 255),
            (0x90, 0),
            (0x8a, 1),
            (0x8b, 1),
            (4, 255),
        ] {
            m.set_u8(a + o, v)?;
        }
        for (o, v) in [
            (6, 0),
            (10, 0),
            (0x7c, 0),
            (0x7e, 0),
            (0x80, 0),
            (0x82, 0),
            (0x84, 200),
            (0x78, 65535),
            (0x86, 65535),
        ] {
            m.set_u16(a + o, v)?;
        }
        m.set_u32(a + 0x8c, 0)?;
    }
    let bags = m.u32(0x4e19b8)?;
    for p in 0..4 {
        let base = m.u32(bags + 4 * p)?;
        for t in 0..26 {
            bag_reset(m, base + 104 * t)?;
        }
    }
    let units = m.u32(0x4e19a0)?;
    for i in 0..800 {
        let u = units + 276 * i;
        if m.u8(u + 2)? != 0 {
            m.set_u16(u + 0x9a, 0)?;
            m.set_u8(u + 2, 0)?;
            m.set_u8(u + 0xaf, 0)?;
            bag_reset(m, u + 10)?;
        }
    }
    let projectiles = m.u32(0x5080f0)?;
    for i in 0..100 {
        m.set_u8(projectiles + 56 * i + 2, 0)?;
    }
    fill(m, 0x4e7754 + 0x20600, 6, 0)?;
    for i in 0..256 {
        m.set_u8(0x4e7754 + 518 * i, 0)?;
    }
    for i in 0..4 {
        let a = 0x4e1a14 + 8 * i;
        fill(m, a, 3, 0)?;
        m.set_u32(a + 4, u32::MAX)?;
    }
    Ok(())
}
fn header(m: &mut MemoriaJogo, r: &mut Reader<'_>) -> Result<()> {
    r.take(4)?;
    for (p, n) in [
        (0x507da8, 65536),
        (0x507d70, 131072),
        (0x507d74, 65536),
        (0x507dac, 65536),
    ] {
        r.read(m, m.u32(p)?, n)?;
    }
    let o = m.u32(0x507d74)?;
    let aux = m.u32(0x507dac)?;
    for c in 0..65536 {
        if m.u8(o + c)? == 0x36 {
            m.set_u8(o + c, 0x42)?;
            m.set_u8(aux + c, 1)?;
        }
    }
    for resource in [1, 2, 0] {
        for p in 0..4 {
            let n = r.word()?;
            // SetMatiere 0x46799c writes both amounts and clears the display/flag fields.
            crate::mission_tail::set_resource(m, p, resource, n)?;
        }
    }
    r.read(m, 0x497168, 4)?;
    for p in 0..4 {
        for part in [0, 2] {
            if m.u8(0x508041)? == p {
                r.read(m, 0x508720 + part, 2)?;
            } else {
                r.take(2)?;
            }
        }
    }
    Ok(())
}
fn units(m: &mut MemoriaJogo, r: &mut Reader<'_>) -> Result<usize> {
    for p in 0..4 {
        m.set_u16(0x508728 + 512 * p, 0)?;
    }
    fill(m, 0x4e19bc, 4, 50)?;
    let count = r.word()?;
    let size = r.word()?;
    require(count <= 800 && size == 784, "LoadEngin header", 0x46acca)?;
    let base = m.u32(0x4e19a0)?;
    for ordinal in 0..u32::from(count) {
        let index = u32::from(r.word()?);
        require(index < 800, "LoadEngin index", 0x46ad24)?;
        let u = base + 276 * index;
        r.read(m, u, 140)?;
        r.take(512)?;
        r.read(m, u + 140, 132)?;
        m.set_u8(base + 276 * ordinal + 0xbd, 0)?;
        if m.u8(u + 2)? == 0 {
            continue;
        }
        frame_spawn::call(m, "EnginSetTypeCaract", &[index])?;
        let t = m.u8(u + 1)?;
        require(t < 26, "LoadEngin type", 0x46adad)?;
        let def = 0x4965ae + 50 * u32::from(t);
        m.set_u16(u + 0x9a, 100)?;
        m.set_u8(u + 0xbb, m.u8(def + 42)?)?;
        m.set_u8(u + 0xb0, m.u8(def + 44)?)?;
        m.set_u8(u + 0xbc, 255)?;
        for o in [0xf5, 0xf6, 0xf7, 0xf8, 0xdc] {
            m.set_u8(u + o, 0)?;
        }
        m.set_u32(u + 0x104, u32::MAX)?;
        m.set_u32(u + 0x10c, u32::MAX)?;
        m.set_u32(u + 0x110, 0)?;
        if m.u8(u + 2)? == 14 {
            m.set_u8(u + 0x72, 0)?;
        }
        m.set_u16(u + 0x102, m.u16(def + 48)?)?;
        match t {
            4 => m.set_u8(u + 0xda, 10)?,
            14 | 16 => m.set_u8(u + 0xda, 0)?,
            _ => (),
        }
        if t == 16 {
            m.set_u8(u, 0)?;
        } else if (20..=24).contains(&t) {
            m.set_u8(u, 4)?;
        }
    }
    Ok(usize::from(count))
}
#[derive(Debug)]
pub struct MissionLoaded {
    pub structures: usize,
    pub units: usize,
    pub file_bytes: usize,
}
/// Successful-open MissionLoad body, including the complete gameplay tail.
/// Use `load` for the reset/open-failure semantics; this continuation is also useful for differential checkpoints.
pub fn load_body(m: &mut MemoriaJogo, eta: &[u8], monsters: &MissionIni) -> Result<MissionLoaded> {
    let mut r = Reader {
        bytes: eta,
        position: 0,
    };
    header(m, &mut r)?;
    crate::mission_terrain::remap(m)?;
    let count = r.word()?;
    let size = r.word()?;
    require(size == 204, "MissionLoad structure size", 0x46c534)?;
    for _ in 0..count {
        let i = u32::from(r.word()?);
        require(i < 1000, "MissionLoad structure index", 0x46c549)?;
        r.read(m, 0x49a0c8 + 276 * i, 204)?;
    }
    let n = units(m, &mut r)?;
    crate::mission_aux::load(m, &mut r, monsters)?;
    m.set_u32(0x4e1d1c, 0)?;
    crate::mission_terrain::place(m)?;
    crate::mission_tail::load_tail(m)?;
    crate::mission_ai::reset(m)?;
    crate::mission_tail::market_reset(m)?;
    Ok(MissionLoaded {
        structures: usize::from(count),
        units: n,
        file_bytes: r.position,
    })
}
/// File bytes are supplied by the host after resolving `mission_name`. Missing file returns false after the original resets.
pub fn load(
    m: &mut MemoriaJogo,
    eta: Option<&[u8]>,
    monsters: &MissionIni,
) -> Result<Option<MissionLoaded>> {
    let _ = mission_name(m, m.u8(0x496fe4)?)?;
    reset(m)?;
    let Some(eta) = eta else {
        m.set_u32(0x4e1d1c, 0)?;
        return Ok(None);
    };
    load_body(m, eta, monsters).map(Some)
}
