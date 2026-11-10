//! Economy S1 research/technology, pinned 3e2ed77. No foreign return replay.
use crate::{frame::stop, player::Memory, FrameRegisters as R, MemoriaJogo, Result};
const RESEARCH: u32 = 0x5080f4;
const TABLE: u32 = 0x50817c;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X87Push {
    pub significand: u64,
    pub sign_exponent: u16,
}
#[derive(Debug)]
pub struct TechnologyResult {
    pub registers: R,
    pub x87_push: Option<X87Push>,
}
pub fn double_to_x87(bits: u64) -> Result<X87Push> {
    let sign = (bits >> 63) as u16;
    let exponent = (bits >> 52) & 0x7ff;
    let fraction = bits & ((1 << 52) - 1);
    let (significand, e) = if exponent == 0x7ff {
        return stop("RechercheGetEtat nonfinite value", 0x4717aa);
    } else if exponent != 0 {
        (
            ((1 << 52) | fraction) << 11,
            (exponent as i32 - 1023 + 16383) as u16,
        )
    } else if fraction != 0 {
        let bit = 63 - fraction.leading_zeros();
        (fraction << (63 - bit), (16383 - 1074 + bit) as u16)
    } else {
        (0, 0)
    };
    Ok(X87Push {
        significand,
        sign_exponent: e | (sign << 15),
    })
}
#[cfg(feature = "validation-trace")]
#[derive(Debug)]
pub struct ForeignCall {
    pub name: &'static str,
    pub site: u32,
    pub input: R,
    pub output: R,
}
#[derive(Default)]
pub struct Technology {
    #[cfg(feature = "validation-trace")]
    pub calls: Vec<ForeignCall>,
}
impl Technology {
    pub fn entry(&mut self, m: &mut MemoriaJogo, name: &str, r: R) -> Result<TechnologyResult> {
        if name == "RechercheGetEtat" {
            let p = r.eax & 255;
            let a = 0x5080fc + 16 * p;
            let bits = u64::from(m.d(a)?) | (u64::from(m.d(a + 4)?) << 32);
            return Ok(TechnologyResult {
                registers: R {
                    eax: 16 * p,
                    esp: r.esp.wrapping_add(4),
                    ..r
                },
                x87_push: Some(double_to_x87(bits)?),
            });
        }
        let mut q = self.body(m, name, r)?;
        q.esp = r.esp.wrapping_add(4);
        Ok(TechnologyResult {
            registers: q,
            x87_push: None,
        })
    }
    fn call(
        &mut self,
        m: &mut MemoriaJogo,
        name: &'static str,
        site: u32,
        mut r: R,
        stack: u32,
    ) -> Result<R> {
        let esp = r.esp;
        r.esp = r.esp.wrapping_sub(stack);
        let mut q = match name {
            "MissionGetNum" => R {
                eax: (r.eax & 0xffff0000) | m.b(0x496fe4)?,
                ..r
            },
            "PlayerGetType" => R {
                eax: crate::frame_economy::call(m, name, &[r.eax])?,
                ..r
            },
            _ => self.entry(m, name, r)?.registers,
        };
        #[cfg(feature = "validation-trace")]
        if matches!(name, "MissionGetNum" | "PlayerGetType") {
            self.calls.push(ForeignCall {
                name,
                site,
                input: r,
                output: R {
                    esp: r.esp.wrapping_add(4),
                    ..q
                },
            });
        }
        #[cfg(not(feature = "validation-trace"))]
        let _ = site;
        q.esp = esp;
        Ok(q)
    }
    fn body(&mut self, m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
        let original = r;
        let p = r.eax & 255;
        let t = r.edx & 255;
        match name {
            "RechercheGetBranche" => r.eax = m.b(RESEARCH + 16 * p + 4)?,
            "RechercheGetNiveau" => r.eax = m.b(RESEARCH + 16 * p + (r.edx & 65535))?,
            "RechercheGetNbInvention" => r.eax = m.b(0x50871c + p)?,
            "InventionGetCout" => {
                let item = r.eax & 65535;
                r.eax = m.d(TABLE + 360 * (r.edx & 65535) + 12 * item)?;
                r.edx = item;
            }
            "SetBatimentRecrutable" => {
                m.sb(0x4dfad4 + 22 * p + t, 1)?;
                r.eax = 11 * p;
                r.edx = t;
            }
            "SetTroupeRecrutable" => {
                m.sb(0x4e1934 + 26 * p + t, 1)?;
                r.eax = 13 * p;
                r.edx = t;
            }
            "TypeEnginEstRealisable" => {
                r.eax = m.b(0x4e1934 + 26 * p + t)?;
                r.edx = 26 * p;
            }
            "TypeBatimentEstRealisable" => {
                if m.b(0x4e0730)? != 0 && matches!(t, 8 | 12) {
                    r.eax = 0;
                } else {
                    r.eax = m.b(0x4dfad4 + 22 * p + t)?;
                    r.edx = 22 * p;
                }
            }
            "SortilegeResetTechnologie" => {
                for i in 0..5 {
                    for a in [
                        0x508f30, 0x508f44, 0x508f35, 0x508f49, 0x508f3a, 0x508f4e, 0x508f3f,
                        0x508f53,
                    ] {
                        m.sb(a + i, 0)?;
                    }
                }
                r.eax = 5;
            }
            "TechnologieReset" => {
                for p in 0..4 {
                    for t in 0..26 {
                        m.sb(0x4e1934 + 26 * p + t, 0)?;
                    }
                }
                for p in 0..4 {
                    for t in 0..22 {
                        m.sb(0x4dfad4 + 22 * p + t, 0)?;
                    }
                }
                r.eax = 22;
                r.ebx = 66;
                r.ecx = 4;
                r.edx = 4;
                r = self.call(m, "SortilegeResetTechnologie", 0x46b576, r, 16)?;
                r.ebx = original.ebx;
                r.ecx = original.ecx;
                r.edx = original.edx;
            }
            "BonusReset" => {
                for p in 0..4 {
                    for t in 0..26 {
                        let a = 0x4dfba0 + 468 * p + 18 * t;
                        for (o, n) in [
                            (0, 2),
                            (2, 1),
                            (4, 2),
                            (12, 1),
                            (6, 1),
                            (11, 1),
                            (8, 2),
                            (10, 1),
                            (13, 1),
                            (14, 1),
                        ] {
                            if n == 2 {
                                m.sw(a + o, 0)?;
                            } else {
                                m.sb(a + o, 0)?;
                            }
                        }
                    }
                }
                r.eax = 4;
            }
            "EnginSetTechnologie" => {
                if p >= 4 {
                    return stop("EnginSetTechnologie protected player", 0x4411f9);
                }
                let pool = m.d(m.d(0x4e19b8)?.wrapping_add(4 * p))?;
                bag_init(m, pool + 7 * 104, 22)?;
                for t in 0..22 {
                    let q = self.body(
                        m,
                        "TypeBatimentEstRealisable",
                        R {
                            eax: p,
                            edx: t,
                            ..r
                        },
                    )?;
                    if q.eax & 255 != 0 && t != 20 {
                        bag_take(m, pool + 7 * 104, 3, t)?;
                    }
                }
                bag_init(m, pool + 14 * 104, 1)?;
                bag_take(m, pool + 14 * 104, 7, 6)?;
                bag_init(m, pool + 8 * 104, 4)?;
                for t in 0..4 {
                    bag_take(m, pool + 8 * 104, 2, t)?;
                }
                r.eax = pool + 8 * 104;
            }
            "BatimentInitTechnologie" => {
                for i in 0..1000 {
                    let a = 0x49a0c8 + 276 * i;
                    if m.b(a + 5)? == 0 {
                        continue;
                    }
                    let t = m.b(a + 8)?;
                    if t == 3 {
                        let p = m.b(a + 4)?;
                        if p < 4 && m.w(0x4dfb2c + 6 * p)? != 0 {
                            crate::chain_habitation::level(m, p, 0x41ce92)?;
                        }
                    }
                    r.eax = m.b(a + 4)?;
                    r.edx = t;
                    r.esi = a;
                    r.edi = i;
                    r = crate::chain_habitation::own(m, "BatimentSetTechnologie", r)?;
                }
                r = R {
                    eax: 1000,
                    ..original
                };
            }
            "InventionSetEffet" => {
                let item = r.edx;
                let branch = r.ebx;
                let offset = 360 * (branch & 65535) + 12 * (item & 65535);
                let a = TABLE + offset;
                let kind = m.b(a + 6)?;
                let valid = kind != 255 && m.b(a + 4)? != 100;
                let subtype = m.b(a + 7)?;
                let target = m.b(a + 8)?;
                let delegated = (!valid && r.ecx & 255 == 1 && p == m.b(0x508041)?)
                    || kind != 0
                    || !matches!(subtype, 1 | 3)
                    || (subtype == 3 && matches!(target, 13 | 16 | 18 | 20))
                    || (subtype == 1 && matches!(target, 11 | 13 | 15));
                if delegated {
                    // The notification, the bonus kinds and the special unlocks
                    // run through the complete transcription (mission_research::effect,
                    // natively proved by claude-20261103/b invention-471017 and
                    // claude-20261109 technology_effect-v1).
                    crate::mission_research::effect(
                        m,
                        p,
                        item & 65535,
                        branch & 65535,
                        r.ecx & 255 == 1,
                    )?;
                    r.esi = original.esi;
                    r.edi = original.edi;
                } else {
                    let subtype = m.b(a + 7)?;
                    let target = m.b(a + 8)?;
                    r.esi = item;
                    r.edi = branch;
                    if subtype == 3 {
                        r.eax = p;
                        r.edx = target;
                        r.ecx = p;
                        r.ebx = offset;
                        r = self.call(m, "SetBatimentRecrutable", 0x471135, r, 16)?;
                        r.eax = r.ecx;
                        r = self.call(m, "EnginSetTechnologie", 0x47113c, r, 16)?;
                        r.eax = (r.eax & 0xffff0000) | (target << 8) | target;
                    } else {
                        r.eax = p;
                        r.edx = target;
                        r.ebx = 3 * (item & 65535);
                        r = self.call(m, "SetTroupeRecrutable", 0x4711be, r, 16)?;
                        r = self.call(m, "BatimentInitTechnologie", 0x4711c3, r, 16)?;
                        r.eax = (target << 8) | target;
                        r.edx = 360 * (branch & 65535);
                        r.ebx = 3 * (item & 65535);
                    }
                    r.esi = original.esi;
                    r.edi = original.edi;
                }
            }
            "RechercheCalculNew" => {
                r = self.call(m, "TechnologieReset", 0x471518, r, 36)?;
                r.edx = 0;
                r = self.call(m, "BonusReset", 0x47151f, r, 36)?;
                for p in 0..4 {
                    for o in [0, 4, 8, 12] {
                        m.sd(RESEARCH + 16 * p + o, m.d(0x508134 + 16 * p + o)?)?;
                    }
                }
                r.ecx = 4;
                r.ebx = 0;
                r.ebp = 0;
                for p in 0..4 {
                    for branch in 0..4 {
                        let mut item = 0;
                        while item < m.b(RESEARCH + 16 * p + branch)? {
                            if item >= 30 {
                                return stop("RechercheCalculNew stored level", 0x4715a7);
                            }
                            if m.b(TABLE + 360 * branch + 12 * item + 6)? != 255 {
                                r.eax = p;
                                r.edx = item;
                                r.ebx = branch;
                                r.ecx = 0;
                                r.esi = item;
                                r.edi = branch;
                                r.ebp = 0;
                                r = self.call(m, "InventionSetEffet", 0x4715df, r, 36)?;
                            }
                            item += 1;
                        }
                    }
                }
                for p in 0..4 {
                    m.sb(0x50871c + p, 0)?;
                    r.edx = 0;
                    r.edi = 0;
                    for branch in 0..4 {
                        let ebp = RESEARCH + 16 * p;
                        r.edi = branch;
                        r.ebp = ebp;
                        let mut ended = false;
                        for item in 0..30 {
                            let offset = 360 * branch + 12 * item;
                            let a = TABLE + offset;
                            let kind = m.b(a + 6)?;
                            r.eax = kind;
                            r.esi = offset;
                            if kind == 255 {
                                ended = true;
                                break;
                            }
                            r.edx = m.b(a + 5)?;
                            r = self.call(m, "MissionGetNum", 0x47165f, r, 36)?;
                            if r.edx > r.eax & 65535 {
                                ended = true;
                                break;
                            }
                            let level = m.b(ebp + branch)?;
                            r.eax = branch;
                            r.edx = level;
                            r.esi = item;
                            if level > item {
                                continue;
                            }
                            let kind = m.b(a + 6)?;
                            r.eax = kind;
                            r.esi = offset;
                            if kind == 255 {
                                continue;
                            }
                            r.edx = m.b(a + 4)?;
                            r = self.call(m, "MissionGetNum", 0x4716b0, r, 36)?;
                            if r.edx <= r.eax & 65535 {
                                r.eax = p;
                                r.edx = item;
                                r.ebx = branch;
                                r.ecx = 1;
                                r.esi = branch;
                                r = self.call(m, "InventionSetEffet", 0x4716df, r, 36)?;
                                r.eax = (r.eax & 0xffffff00) | ((item + 1) & 255);
                                m.sb(ebp + branch, r.eax)?;
                            }
                        }
                        if !ended {
                            return stop("RechercheCalculNew unterminated branch", 0x471632);
                        }
                    }
                }
                r = R { eax: 4, ..original };
            }
            "IaRechercheAjuste" => {
                r.eax = 0;
                r.edx = 1;
                r = self.call(m, "RechercheGetNbInvention", 0x47fa98, r, 32)?;
                r.ebp = r.eax & 255;
                for p in 1..4 {
                    r.eax = p;
                    r.edx = p;
                    r.ebx = 0;
                    r = self.call(m, "PlayerGetType", 0x47fabe, r, 32)?;
                    r.ebx = r.eax & 255;
                    if r.ebx != 2 {
                        continue;
                    }
                    r.ebx = 0;
                    r = self.call(m, "MissionGetNum", 0x47facc, r, 32)?;
                    r.ebx = r.eax & 65535;
                    if r.ebx >= 3 {
                        return stop("IaRechercheAjuste AI research catch-up", 0x47fad9);
                    }
                }
                r = R {
                    eax: r.eax,
                    ..original
                };
            }
            _ => return stop("unknown S1 technology routine", 0),
        }
        Ok(r)
    }
}
fn bag_init(m: &mut MemoriaJogo, a: u32, cap: u32) -> Result<()> {
    for o in [0x66, 4, 2, 0] {
        m.sw(a + o, 0)?;
    }
    for i in 0..30 {
        m.sw(a + 6 + 2 * i, 0)?;
        m.sb(a + 0x42 + i, 0)?;
    }
    m.sw(a + 2, cap)
}
fn bag_take(m: &mut MemoriaJogo, a: u32, tag: u32, value: u32) -> Result<u32> {
    let n = m.w(a)?;
    if n >= 30 {
        return stop("BagPrend protected count", 0x447483);
    }
    m.sw(a + 0x66, m.w(a + 0x66)?.wrapping_add(1))?;
    m.sb(a + 0x42 + n, tag)?;
    m.sw(a + 6 + 2 * n, value)?;
    m.sw(a, n + 1)?;
    Ok(n)
}

/// RechercheCalcul 0x47132c: InventionSetEffet(AL=player, DL=item, BL=branch,
/// ECX=1). The frame caller consumes effects only; the whole original routine,
/// including its completion notification, is mission_research::effect.
pub fn apply_invention(m: &mut MemoriaJogo, player: u32, item: u32, branch: u32) -> Result<()> {
    crate::mission_research::effect(m, player, item, branch, true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_push_preserves_negative_zero_and_subnormal_without_host_rounding() {
        assert_eq!(
            double_to_x87(1 << 63).unwrap(),
            X87Push {
                significand: 0,
                sign_exponent: 0x8000
            }
        );
        assert_eq!(
            double_to_x87(1).unwrap(),
            X87Push {
                significand: 1 << 63,
                sign_exponent: 15309
            }
        );
        assert_eq!(
            double_to_x87(0x3ff0000000000000).unwrap(),
            X87Push {
                significand: 1 << 63,
                sign_exponent: 16383
            }
        );
        assert!(matches!(
            double_to_x87(0x7ff0000000000000),
            Err(crate::EngineError::UnportedCallee {
                address: 0x4717aa,
                ..
            })
        ));
    }
}
