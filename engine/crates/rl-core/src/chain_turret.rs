//! Combat OrdensTorresCombate, original 0x4242cc/0x42436c.
use crate::{
    frame::stop,
    player::{structure, unit, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
pub fn entry(m: &mut MemoriaJogo, n: &str, mut r: R) -> Result<R> {
    if n == "BatimentInitAttaqueSelect" {
        let p = r.eax & 255;
        if p >= 4 {
            return stop("Turret player", 0x4242cc);
        }
        let s = m.w(0x495be0 + 2 * p)?;
        if s >= 1000 {
            return stop("Turret selected structure", 0x4242cc);
        }
        let mode = r.edx & 255;
        let reference = r.ebx & 65535;
        let a = structure(s);
        if mode != 3 || reference != s {
            m.sw(a + 0x86, reference)?;
            m.sb(a + 0x7b, mode)?;
        }
        r.eax = a;
        r.ebx = mode;
        r.edx = 2 * p;
        return Ok(r);
    }
    let s = r.eax & 65535;
    if s >= 1000 {
        return stop("Turret source structure", 0x42436c);
    }
    let a = structure(s);
    let kind = m.b(a + 8)?;
    let mode = r.edx & 255;
    let target = r.ebx & 65535;
    r.eax = 0;
    r.ebx = a;
    r.edx = s;
    if !matches!(kind, 10 | 11) {
        return Ok(r);
    }
    if !matches!(mode, 1 | 3 | 5) {
        return stop("Turret uninitialized target mode", 0x424590);
    }
    if mode == 1 {
        if target >= 800 {
            return stop("Turret target unit", 0x42436c);
        }
        let u = unit(m, target)?;
        r.edx = target;
        if m.b(u)? == 200 {
            return Ok(r);
        }
        if matches!(m.b(u + 1)?, 16 | 17) {
            r.edx = s;
            if kind == 10 {
                return Ok(r);
            }
        }
    } else if mode == 3 {
        if target >= 1000 {
            return stop("Turret target structure", 0x42436c);
        }
        r.edx = target;
        if m.b(structure(target) + 4)? == 200 {
            return Ok(r);
        }
    }
    let origin = m.w(a + 2)?;
    let owner = m.b(a + 4)?;
    let bonus = crate::frame_economy::bonus(m, owner, 11, 4, 7)?;
    r.eax = bonus & !255;
    r.ebx = mode;
    r.edx = if matches!(owner, 4 | 200) { 4 } else { 11 };
    if mode == 5 {
        return Ok(r);
    }
    let c1 = if mode == 1 {
        m.w(unit(m, target)? + 0x7c)?
    } else {
        crate::frame_core::building_cell(m, target)? & 65535
    };
    let c2 = if mode == 1 {
        m.w(unit(m, target)? + 0x7c)?
    } else {
        crate::frame_core::building_cell(m, target)? & 65535
    };
    let dx = (origin & 255).abs_diff(c1 & 255);
    let dy = (origin >> 8).abs_diff(c2 >> 8);
    let d = dx * dx + dy * dy;
    r.ebx = if origin >> 8 > c2 >> 8 { c2 >> 8 } else { dy };
    r.edx = dy * dy;
    let radius = (7 + bonus) & 65535;
    r.eax = (d & !255) | u32::from(d <= radius * radius);
    Ok(r)
}
