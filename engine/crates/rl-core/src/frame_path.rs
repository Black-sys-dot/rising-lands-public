//! Chemin_Verouille/Calcul/Optimise on the original live grids and scratch.
use crate::{frame_core, frame_spawn, EngineError, MemoriaJogo, Result};
fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}
pub const CLASSES: [u8; 26] = [
    1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 1, 1, 1, 1, 1, 1, 1, 3, 3,
];
pub fn vector(d: u8) -> (i32, i32) {
    match d & 15 {
        1 => (-1, 0),
        3 => (-1, -1),
        5 => (0, -1),
        7 => (1, -1),
        9 => (1, 0),
        11 => (1, 1),
        13 => (0, 1),
        15 => (-1, 1),
        _ => (0, 0),
    }
}
pub fn direction(a: u16, b: u16, equal_five: bool) -> u8 {
    let dx = (i32::from(b & 255) - i32::from(a & 255)).signum();
    let dy = (i32::from(b >> 8) - i32::from(a >> 8)).signum();
    match (dx, dy) {
        (-1, 0) => 1,
        (-1, -1) => 3,
        (0, -1) => 5,
        (1, -1) => 7,
        (1, 0) => 9,
        (1, 1) => 11,
        (0, 1) => 13,
        (-1, 1) => 15,
        _ => {
            if equal_five {
                5
            } else {
                0
            }
        }
    }
}
pub fn neighbor(m: &MemoriaJogo, c: u16, d: u8) -> Result<u16> {
    let (dx, dy) = vector(d);
    let x = i32::from(c & 255) + dx;
    let y = i32::from(c >> 8) + dy;
    Ok(
        if x >= 0 && x < i32::from(m.u16(0x497168)?) && y >= 0 && y < i32::from(m.u16(0x49716a)?) {
            (x + 256 * y) as u16
        } else {
            65535
        },
    )
}
#[derive(Clone, Copy)]
struct Context {
    unit: u32,
    class: u8,
    mask: u8,
    owner: u8,
}
fn context(m: &MemoriaJogo, i: u32) -> Result<Context> {
    let u = frame_spawn::unit(m, i)?;
    let t = m.u8(u + 1)?;
    let class = *CLASSES
        .get(t as usize)
        .ok_or(stop("Chemin unit type", 0x43b30b))?;
    Ok(Context {
        unit: i,
        class,
        mask: 1 + u8::from(m.u8(u + 0xab)? != 0),
        owner: m.u8(u)?,
    })
}
fn grid8(m: &MemoriaJogo, p: u32, c: u16) -> Result<u8> {
    m.u8(m.u32(p)?.wrapping_add(u32::from(c)))
}
fn grid16(m: &MemoriaJogo, p: u32, c: u16) -> Result<u16> {
    m.u16(m.u32(p)?.wrapping_add(2 * u32::from(c)))
}
pub(crate) fn allied(m: &MemoriaJogo, a: u8, b: u8) -> Result<bool> {
    if a == b {
        return Ok(true);
    }
    if a > 4 || b > 4 {
        return Ok(false);
    }
    let p = m.u32(0x5080a8)?.wrapping_add(20 * u32::from(a) + 6);
    Ok(m.u8(p)? & (1 << b) != 0)
}
fn mask_cell(m: &MemoriaJogo, c: u16, mask: u8) -> Result<bool> {
    Ok(if grid8(m, 0x507d74, c)? == 0xfd {
        mask & 1 != 0
    } else {
        grid8(m, 0x507da8, c)? & mask != 0
    })
}
fn own(m: &MemoriaJogo, c: Context, cell: u16) -> Result<bool> {
    let u = frame_spawn::unit(m, c.unit)?;
    let ab = m.u8(u + 0xab)?;
    if (ab == 50 && c.mask == 1) || (ab == 0 && c.mask == 2) || m.u8(u + 2)? == 14 {
        return Ok(false);
    }
    let current = i32::from(m.u16(u + 8)?);
    let cell = i32::from(cell);
    Ok(if c.class == 1 {
        current == cell
    } else {
        [0, 1, 257, 256].iter().any(|d| current - d == cell)
    })
}
fn occupant_pass(m: &MemoriaJogo, c: Context, other: u32, d: u8) -> Result<bool> {
    let u = frame_spawn::unit(m, c.unit)?;
    let q = frame_spawn::unit(m, other)?;
    let t = m.u8(u + 1)?;
    let ot = m.u8(q + 1)?;
    if t >= 26 || ot >= 26 {
        return Err(stop("Chemin classify unit type", 0x42ece0));
    }
    let mut difference = (i32::from(m.u8(q + 0x90)?) - i32::from(d)) / 2;
    if difference < -4 {
        difference += 8;
    }
    if difference > 4 {
        difference -= 8;
    }
    let aligned = (-1..=1).contains(&difference);
    let mut moving = m.u16(q + 8)? != m.u16(q + 0x7e)?;
    if (matches!(ot, 0 | 1 | 20 | 21) && m.u8(q + 2)? == 1 && m.u8(q + 0x8a)? == 1)
        || (matches!(ot, 6 | 7) && m.u8(q + 2)? == 15 && m.u8(q + 0x89)? == 1)
    {
        moving = false;
    }
    let friend = allied(m, c.owner, m.u8(q)?)?;
    let exception =
        matches!(t, 12 | 15) && matches!(ot, 0 | 1 | 2 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 18 | 19 | 22);
    Ok(if friend {
        if moving {
            aligned
        } else {
            m.u8(0x4e1771)? != 0
        }
    } else {
        exception || (moving && aligned)
    })
}
fn passage(m: &MemoriaJogo, c: Context, cell: u16, d: u8) -> Result<bool> {
    if !matches!(c.class, 1..=3) {
        return Ok(true);
    }
    if c.class != 1 && (cell & 255 == 0 || cell >> 8 == 0) {
        return Ok(false);
    }
    for delta in if c.class == 1 {
        &[0u16][..]
    } else {
        &[0, 1, 257, 256][..]
    } {
        let cell = cell.wrapping_sub(*delta);
        if !mask_cell(m, cell, c.mask)? || own(m, c, cell)? {
            continue;
        }
        let v = grid16(m, 0x4e0f38, cell)?;
        if v & 0xc000 != 0x4000 {
            return Ok(false);
        }
        if !occupant_pass(m, c, u32::from((v & 0x3ff8) >> 3), d)? {
            return Ok(false);
        }
    }
    Ok(true)
}
#[derive(Clone)]
struct Preparation {
    start: u16,
    destination: u16,
    alternative: u16,
    status: u8,
    done: u8,
    success: u8,
}
fn rings(dest: u16, r: i32, w: i32, h: i32) -> Vec<u16> {
    let x = i32::from(dest & 255);
    let y = i32::from(dest >> 8);
    let upper = i32::from(dest) - r - r * 256;
    let mut out = Vec::new();
    if y - r >= 0 {
        for j in 0..=2 * r {
            if 0 <= x + j - r && x + j - r <= w {
                out.push((upper + j) as u16);
            }
        }
    }
    if y + r < h {
        for j in 0..=2 * r {
            if 0 <= x + j - r && x + j - r <= w {
                out.push((upper + 2 * r * 256 + j) as u16);
            }
        }
    }
    if x - r >= 0 {
        for j in 1..2 * r {
            if y + j - r >= 0 && x + j - r < h {
                out.push((upper + (j - 1) * 256) as u16);
            }
        }
    }
    if x + r < w {
        for j in 1..2 * r {
            if y + j - r >= 0 && x + j - r < h {
                out.push((upper + 2 * r + (j - 1) * 256) as u16);
            }
        }
    }
    out
}
fn prepare(m: &MemoriaJogo, c: Context, start: u16, requested: u16) -> Result<Preparation> {
    let mut dest = requested;
    let mut alt = 65535;
    let mut status = 1;
    if !passage(m, c, dest, direction(start, dest, true))? {
        status = 2;
        let w = i32::from(m.u16(0x497168)?);
        let h = i32::from(m.u16(0x49716a)?);
        dest = if dest == start {
            start
        } else if i32::from(dest & 255) >= w || i32::from(dest >> 8) >= h {
            65535
        } else {
            let dist = (i32::from(dest & 255) - i32::from(start & 255))
                .abs()
                .max((i32::from(dest >> 8) - i32::from(start >> 8)).abs());
            let d = direction(start, dest, true);
            let mut found = 65535;
            'search: for r in 1..30 {
                if dist == r {
                    found = start;
                    break;
                }
                for cc in rings(dest, r, w, h) {
                    if passage(m, c, cc, d)? {
                        found = cc;
                        break 'search;
                    }
                }
            }
            found
        };
        alt = dest;
    }
    let (done, success) = if !frame_spawn::accessible(m, start, dest)? {
        status = 7;
        (1, 0)
    } else if dest == 65535 {
        status = 6;
        (1, 0)
    } else if start == dest {
        (1, 1)
    } else {
        (0, 0)
    };
    Ok(Preparation {
        start,
        destination: dest,
        alternative: alt,
        status,
        done,
        success,
    })
}
fn get16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn put16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn observe(
    m: &MemoriaJogo,
    c: Context,
    value: u16,
    cell: u16,
    record: &mut [u8; 10],
    phase: u8,
) -> Result<u16> {
    let i = u32::from((value & 0x7fe0) >> 5);
    if i >= 1000 {
        return Err(stop("Chemin structure index", 0x42f214));
    }
    let b = 0x49a0c8 + 276 * i;
    let enemy = u8::from(!allied(m, m.u8(b + 4)?, c.owner)?);
    if m.u8(b + 8)? == 9 && (record[5] == 0 || enemy > record[7]) {
        put16(record, 2, i as u16);
        record[5] = 1;
        record[7] = enemy;
        record[9] = phase;
    }
    Ok(0x8000 | u16::from(grid8(m, 0x4e156c, cell)?))
}
#[allow(clippy::too_many_arguments)] // Original contour record and region-list state.
fn add_region(
    m: &MemoriaJogo,
    c: Context,
    v: u16,
    cell: u16,
    record: &mut [u8; 10],
    phase: u8,
    dedup: bool,
    origin: u16,
    regions: &mut Vec<u16>,
) -> Result<()> {
    if v == 0 || v == origin {
        return Ok(());
    }
    if v & 0x8000 != 0 {
        let v = observe(m, c, v, cell, record, phase)?;
        if (!dedup || !regions.contains(&v)) && regions.len() < 255 {
            regions.push(v);
        }
        return Ok(());
    }
    if dedup && regions.contains(&v) {
        return Ok(());
    }
    if v & 0x4000 == 0 {
        if regions.len() < 255 {
            regions.push(v);
        }
        return Ok(());
    }
    let other = u32::from((v & 0x3ff8) >> 3);
    if other == c.unit {
        return Ok(());
    }
    let enemy = u8::from(!allied(m, m.u8(frame_spawn::unit(m, other)?)?, c.owner)?);
    if record[4] == 0 || enemy > record[6] {
        put16(record, 0, other as u16);
        record[4] = 1;
        record[6] = enemy;
        record[8] = phase;
    }
    if v & 4 != 0 {
        if regions.len() < 251 {
            for j in 0..4 {
                regions.push((v & 0xfffc) + j);
            }
        }
    } else if regions.len() < 255 {
        regions.push(v);
    }
    Ok(())
}
struct Candidates {
    start: u16,
    destination: u16,
    origin: u16,
    obstacle: u16,
    all_blocked: bool,
    regions: Vec<u16>,
    record: [u8; 10],
}
fn candidates(m: &MemoriaJogo, c: Context, p: &Preparation) -> Result<Candidates> {
    let start = p.start;
    let d = direction(start, p.destination, true);
    let origin = m.u16(frame_spawn::unit(m, c.unit)? + 0xfa)?;
    let mut obstacle = 65535;
    let mut best = 1;
    let mut all = true;
    let mut regions = Vec::new();
    let mut record = [0; 10];
    let big = [
        (-2, 1),
        (-2, 0),
        (-2, -1),
        (-2, -2),
        (-1, -2),
        (0, -2),
        (1, -2),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
    ];
    let desired = if c.class == 1 {
        i32::from(d)
    } else {
        [1, 3, 5, 7, 8, 9, 10, 11][usize::from(d / 2)]
    };
    let circle = if c.class == 1 { 16 } else { 12 };
    let neighbors = if c.class == 1 {
        (1..16u8)
            .step_by(2)
            .map(|d| {
                let (x, y) = vector(d);
                (i32::from(d), x, y)
            })
            .collect::<Vec<_>>()
    } else {
        big.into_iter()
            .enumerate()
            .map(|(i, (x, y))| (i as i32, x, y))
            .collect()
    };
    for (direction, dx, dy) in neighbors {
        let x = i32::from(start & 255) + dx;
        let y = i32::from(start >> 8) + dy;
        if x < 0 || x >= i32::from(m.u16(0x497168)?) || y < 0 || y >= i32::from(m.u16(0x49716a)?) {
            continue;
        }
        let cell = (x + y * 256) as u16;
        if cell == 65535 {
            continue;
        }
        if passage(m, c, cell, direction as u8)? {
            all = false;
            continue;
        }
        let mut diff = direction - desired;
        if diff < -circle / 2 {
            diff += circle;
        }
        if diff > circle / 2 {
            diff -= circle;
        }
        let dist = diff.abs();
        if obstacle == 65535 || dist < best {
            obstacle = cell;
            best = dist;
        }
        add_region(
            m,
            c,
            grid16(m, 0x4e0f38, cell)?,
            cell,
            &mut record,
            0,
            true,
            origin,
            &mut regions,
        )?;
    }
    Ok(Candidates {
        start,
        destination: p.destination,
        origin,
        obstacle,
        all_blocked: all,
        regions,
        record,
    })
}
fn subcells(c: Context, cell: u16) -> Vec<u16> {
    if c.class == 1 {
        return vec![0];
    }
    let mut d = vec![0];
    if cell & 255 != 0 {
        d.push(1);
    }
    if cell > 255 {
        d.push(256);
    }
    if cell & 255 != 0 && cell > 256 {
        d.push(257);
    }
    d
}
#[allow(clippy::too_many_arguments)] // Original contour context and mutable record.
fn line_out(
    m: &MemoriaJogo,
    c: Context,
    mut at: u16,
    dest: u16,
    regions: &mut Vec<u16>,
    record: &mut [u8; 10],
    phase: u8,
    origin: u16,
) -> Result<bool> {
    let mut d = direction(at, dest, true);
    let first = neighbor(m, at, d)?;
    if !passage(m, c, first, d)? {
        let mut found = false;
        for delta in subcells(c, first) {
            let v = grid16(m, 0x4e0f38, first.wrapping_sub(delta))?;
            if v & 0xc000 == 0x4000 {
                add_region(m, c, v, 0, record, phase, false, origin, regions)?;
                found = true;
            }
        }
        if found {
            return Ok(false);
        }
    }
    for _ in 0..255 {
        if at == dest {
            return Ok(true);
        }
        if !passage(m, c, at, d)? {
            for delta in subcells(c, at) {
                let cell = at.wrapping_sub(delta);
                let mut v = grid16(m, 0x4e0f38, cell)?;
                if v != origin {
                    if v & 0x8000 != 0 {
                        v = observe(m, c, v, cell, record, phase)?;
                    }
                    if regions.contains(&v) {
                        return Ok(false);
                    }
                }
            }
        }
        d = direction(at, dest, true);
        at = neighbor(m, at, d)?;
        if at == 65535 {
            return Ok(false);
        }
    }
    Ok(true)
}
fn contour_step(
    m: &MemoriaJogo,
    c: Context,
    at: u16,
    obstacle: u16,
    sense: i32,
) -> Result<(u16, u16)> {
    let mut d = direction(at, obstacle, true);
    let mut next = neighbor(m, at, d)?;
    let mut previous = next;
    for _ in 0..10 {
        if passage(m, c, next, d)? {
            return Ok((next, previous));
        }
        previous = next;
        d = (i32::from(d) + 2 * sense).rem_euclid(16) as u8;
        next = neighbor(m, at, d)?;
        if next == obstacle {
            break;
        }
    }
    Ok((65535, previous))
}
fn left_region(m: &MemoriaJogo, c: Context, previous: u16, chosen: u16) -> Result<(u16, u16)> {
    if c.class == 1 {
        return Ok((grid16(m, 0x4e0f38, previous)?, previous));
    }
    for d in [None, Some(1), Some(3), Some(5)] {
        let cell = if let Some(d) = d {
            neighbor(m, previous, d)?
        } else {
            previous
        };
        let delta = i32::from(chosen) - i32::from(cell);
        if mask_cell(m, cell, c.mask)? && ![0, 1, 257, 256].contains(&delta) {
            return Ok((grid16(m, 0x4e0f38, cell)?, cell));
        }
    }
    Ok((0, previous))
}
struct Contour {
    buffers: [Vec<u8>; 2],
    records: [[u8; 10]; 2],
    regions: Vec<u8>,
    count: u8,
    phase: u8,
    costs: [u16; 2],
    failure: Option<u8>,
    steps: usize,
}
fn contours(m: &MemoriaJogo, c: Context, start: &Candidates) -> Result<Contour> {
    let mut b = [
        m.read(0x4e116a, 512)?.to_vec(),
        m.read(0x4e136a, 512)?.to_vec(),
    ];
    for buf in &mut b {
        put16(buf, 0, 0);
    }
    let mut region_data = m.read(0x4e0f6a, 510)?.to_vec();
    let mut records = [
        start.record,
        m.read(0x4e0f5e, 10)?.try_into().expect("record"),
    ];
    let mut costs = [0u16; 2];
    let mut phase = 0u8;
    let mut regions = start.regions.clone();
    let mut steps = 0;
    for (i, r) in regions.iter().enumerate() {
        put16(&mut region_data, i * 2, *r);
    }
    if start.all_blocked {
        return Ok(Contour {
            buffers: b,
            records,
            regions: region_data,
            count: regions.len() as u8,
            phase: 0,
            costs,
            failure: Some(8),
            steps: 0,
        });
    }
    for side in 0..2 {
        if side == 1 {
            records[1] = [0; 10];
        }
        let sense = if side == 0 { -1 } else { 1 };
        let mut at = start.start;
        let mut obstacle = start.obstacle;
        regions = start.regions.clone();
        phase = 0;
        let mut count = 0;
        loop {
            let (next, previous) = contour_step(m, c, at, obstacle, sense)?;
            steps += 1;
            if next == 65535 || previous == 65535 {
                costs[side] = 65535;
            } else {
                let (v, cell) = left_region(m, c, previous, next)?;
                add_region(
                    m,
                    c,
                    v,
                    cell,
                    &mut records[side],
                    phase,
                    true,
                    start.origin,
                    &mut regions,
                )?;
                obstacle = previous;
                at = next;
                phase = phase.wrapping_add(1);
                costs[side] = costs[side].wrapping_add(1);
                count += 1;
                put16(&mut b[side], count * 2, at);
            }
            let exited = line_out(
                m,
                c,
                at,
                start.destination,
                &mut regions,
                &mut records[side],
                phase,
                start.origin,
            )?;
            // The original region table preserves writes beyond the next side's count.
            for (i, r) in regions.iter().enumerate() {
                put16(&mut region_data, i * 2, *r);
            }
            if exited || costs[side] == 65535 || count >= 255 || next == start.destination {
                break;
            }
        }
        if at != start.destination {
            let d = direction(at, start.destination, true);
            let next = neighbor(m, at, d)?;
            if passage(m, c, next, d)?
                && costs[side] != 65535
                && count < 255
                && next != start.destination
            {
                count += 1;
                put16(&mut b[side], count * 2, next);
                phase = phase.wrapping_add(1);
            }
        }
        put16(&mut b[side], 0, count as u16);
    }
    let failure = if costs == [65535, 65535] {
        Some(if b.iter().all(|b| get16(b, 0) == 0) {
            8
        } else {
            6
        })
    } else {
        None
    };
    Ok(Contour {
        buffers: b,
        records,
        regions: region_data,
        count: regions.len() as u8,
        phase,
        costs,
        failure,
        steps,
    })
}
fn select(m: &mut MemoriaJogo, c: &mut Contour, status: &mut u8) -> Result<usize> {
    for i in 0..2 {
        if c.costs[i] == 65535 {
            put16(&mut c.buffers[i], 0, 255);
        }
    }
    if c.buffers.iter().all(|b| get16(b, 0) == 255) {
        for (field, counter, s, flag) in [(4, 8, 4, m.u8(0x4e0f34)?), (5, 9, 5, m.u8(0x4e0f35)?)] {
            if flag != 0 {
                continue;
            }
            for i in 0..2 {
                if c.costs[i] != 65535 && c.records[i][field] != 0 {
                    let n = u16::from(c.records[i][counter]);
                    put16(&mut c.buffers[1], 0, n);
                    *status = s;
                    return Ok(i);
                }
            }
        }
    }
    let selected = if c.costs[0] == c.costs[1] {
        usize::from(frame_core::random(m)? & 3 != 0)
    } else {
        usize::from(c.costs[1] < c.costs[0])
    };
    if get16(&c.buffers[selected], 0) == 255 {
        *status = 3;
    }
    Ok(selected)
}
fn line(m: &mut MemoriaJogo, c: Context, origin: u16, dest: u16) -> Result<(bool, Vec<u16>)> {
    m.set_u8(0x4e1771, 0)?;
    let mut d = direction(origin, dest, true);
    if !passage(m, c, neighbor(m, origin, d)?, d)? {
        return Ok((false, vec![]));
    }
    let mut at = origin;
    let mut cells = Vec::new();
    for _ in 0..255 {
        if at == dest {
            break;
        }
        if !passage(m, c, at, d)? {
            return Ok((false, cells));
        }
        cells.push(at);
        d = direction(at, dest, true);
        at = neighbor(m, at, d)?;
        if at == 65535 {
            return Ok((false, cells));
        }
    }
    Ok((true, cells))
}
fn optimize(m: &mut MemoriaJogo, c: Context, buffer: &[u8]) -> Result<Vec<u8>> {
    let n = usize::from(get16(buffer, 0));
    if n > 255 {
        return Err(stop("OptimiseChemin count", 0x42e890));
    }
    let cells = (0..n).map(|i| get16(buffer, 2 + 2 * i)).collect::<Vec<_>>();
    let mut compact = Vec::new();
    let mut cursor = 0;
    while cursor < cells.len() {
        // Native42e8b8 rejects end <= cursor+1: the immediate successor
        // must get its own outer iteration and opportunity to close a loop.
        for end in ((cursor + 2)..cells.len()).rev() {
            let a = cells[cursor];
            let b = cells[end];
            if ((b as u8).wrapping_add(1).wrapping_sub(a as u8)) < 3
                && (((b >> 8) as u8)
                    .wrapping_add(1)
                    .wrapping_sub((a >> 8) as u8))
                    < 3
            {
                compact.push(a);
                cursor = end;
                break;
            }
        }
        compact.push(cells[cursor]);
        cursor += 1;
    }
    let mut out = buffer.to_vec();
    put16(&mut out, 0, 0);
    let mut cursor = 0;
    let mut count = 0;
    while cursor < compact.len() {
        let mut found = false;
        if compact.len() >= 2 {
            for end in ((cursor + 1)..compact.len() - 1).rev() {
                let (ok, writes) = line(m, c, compact[cursor], compact[end])?;
                if count + writes.len() > 255 {
                    return Err(stop("OptimiseChemin buffer capacity", 0x42e890));
                }
                for (i, v) in writes.iter().enumerate() {
                    put16(&mut out, 2 * (count + i + 1), *v);
                }
                if ok {
                    cursor = end;
                    count += writes.len();
                    found = true;
                    break;
                }
            }
        }
        if !found {
            count += 1;
            if count > 255 {
                return Err(stop("OptimiseChemin buffer capacity", 0x42e890));
            }
            put16(&mut out, 2 * count, compact[cursor]);
            cursor += 1;
        }
        put16(&mut out, 0, count as u16);
    }
    Ok(out)
}
pub fn route(m: &MemoriaJogo, i: u32) -> Result<u32> {
    m.u32(m.u32(0x4e1924)?.wrapping_add((i & 65535) * 4))
}
pub fn calculate(m: &mut MemoriaJogo) -> Result<u32> {
    if m.u8(0x4e0f36)? == 0 || m.u8(0x4e0f37)? != 0 || m.u8(0x496302)? != 0 {
        return Ok(0);
    }
    let i = u32::from(m.u16(0x496304)?);
    let u = frame_spawn::unit(m, i)?;
    let c = context(m, i)?;
    if !matches!(c.class, 1..=3) {
        return Err(stop("CheminCalcul occupation class", 0x42f214));
    }
    let p = prepare(m, c, m.u16(u + 8)?, m.u16(u + 0x7e)?)?;
    let output = route(m, i)?;
    m.set_u16(output, 0)?;
    let mut status = p.status;
    let mut success = p.success;
    if p.done == 0 {
        m.set_u16(0x4e0f44, p.start)?;
        m.set_u16(0x4e0f46, p.destination)?;
        let initial = candidates(m, c, &p)?;
        let mut cont = contours(m, c, &initial)?;
        if let Some(f) = cont.failure {
            status = f;
            success = 0;
        } else {
            let selected = select(m, &mut cont, &mut status)?;
            cont.buffers[selected] = optimize(m, c, &cont.buffers[selected])?;
            let n = usize::from(get16(&cont.buffers[selected], 0));
            m.set_u16(output, n as u16)?;
            for j in 0..n {
                m.set_u16(
                    output.wrapping_add(2 + 2 * j as u32),
                    get16(&cont.buffers[selected], 2 + 2 * (n - j - 1)),
                )?;
            }
            success = 1;
        }
        for (base, b) in [0x4e116a, 0x4e136a].into_iter().zip(&cont.buffers) {
            m.write(base, b)?;
        }
        m.write(0x4e0f6a, &cont.regions)?;
        for (base, r) in [0x4e0f54, 0x4e0f5e].into_iter().zip(cont.records) {
            m.write(base, &r)?;
        }
        m.set_u16(0x4e1772, m.u16(u + 0xfa)?)?;
        m.set_u8(0x4e0f40, cont.phase)?;
        m.set_u8(0x4e0f68, cont.count)?;
        m.set_u32(
            0x4e0f50,
            if cont.failure == Some(8) && cont.steps == 0 {
                0x4e0f54
            } else {
                0x4e0f5e
            },
        )?;
    }
    m.set_u8(0x4e0f41, c.owner)?;
    m.set_u8(0x4e0f48, m.u8(0x4962c8 + u32::from(m.u8(u + 1)?))?)?;
    m.set_u8(0x4e0f42, c.mask)?;
    m.set_u8(0x4e0f34, 0)?;
    m.set_u8(0x4e0f35, 0)?;
    m.set_u8(0x4e0f4c, status)?;
    m.set_u16(0x4e0f4a, p.alternative)?;
    m.set_u16(0x496304, i as u16)?;
    m.set_u8(0x4e0f36, 1)?;
    m.set_u8(0x4e0f37, success)?;
    m.set_u32(0x4e0f3c, output)?;
    m.set_u8(0x496302, 1)?;
    Ok(0)
}
pub fn lock(
    m: &mut MemoriaJogo,
    i: u32,
    start: u16,
    dest: u16,
    mask: u8,
    route: u32,
    flags: [u8; 2],
) -> Result<u32> {
    let mut c = context(m, i)?;
    c.mask = mask;
    let p = prepare(m, c, start, dest)?;
    for (a, v) in [
        (0x4e0f34, flags[0]),
        (0x4e0f35, flags[1]),
        (0x4e0f36, 1),
        (0x4e0f37, p.success),
        (0x496302, p.done),
        (0x4e0f42, mask),
        (0x4e0f48, c.class),
        (0x4e0f41, c.owner),
        (0x4e0f4c, p.status),
    ] {
        m.set_u8(a, v)?;
    }
    m.set_u32(0x4e0f3c, route)?;
    m.set_u16(0x496304, i as u16)?;
    m.set_u16(route, 0)?;
    m.set_u16(0x4e0f4a, p.alternative)?;
    if p.done == 0 {
        m.set_u16(0x4e0f44, p.start)?;
        m.set_u16(0x4e0f46, p.destination)?;
    }
    Ok(u32::from(p.done == 0 || p.success != 0))
}
pub fn call(m: &mut MemoriaJogo, name: &str, a: &[u32]) -> Result<u32> {
    match name {
        "CheminCalcul" => calculate(m),
        "OptimiseChemin" => {
            let c = context(m, a[0])?;
            let buffer = m.read(a[1], 512)?.to_vec();
            let out = optimize(m, c, &buffer)?;
            m.write(a[1], &out)?;
            Ok(0)
        }
        "Chemin_Verouille" => lock(
            m,
            a[2],
            a[0] as u16,
            a[1] as u16,
            a[3] as u8,
            a[4],
            [a[5] as u8, a[6] as u8],
        ),
        _ => Err(stop("Core path provider", 0x42f214)),
    }
}

pub fn begin(m: &mut MemoriaJogo, i: u32) -> Result<u32> {
    m.set_u16(0x496304, i as u16)?;
    m.set_u8(0x4e0f36, 1)?;
    m.set_u8(0x4e0f37, 0)?;
    m.set_u8(0x496302, 0)?;
    calculate(m)
}
pub fn test_line(m: &mut MemoriaJogo, i: u32, start: u16, end: u16, output: u32) -> Result<u32> {
    let c = context(m, i)?;
    let (ok, cells) = line(m, c, start, end)?;
    for (j, v) in cells.iter().enumerate() {
        m.set_u16(output.wrapping_add(2 * j as u32), *v)?;
    }
    Ok(u32::from(ok))
}
