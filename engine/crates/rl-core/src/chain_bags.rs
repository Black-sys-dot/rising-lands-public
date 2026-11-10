//! Core Bag operations, mechanically ported from entrada_estruturas at54d148d.
use crate::{frame::stop, player::Memory, FrameRegisters as R, MemoriaJogo, Result};
fn lo(v: u32, w: u32) -> u32 {
    (v & 0xffff0000) | (w & 65535)
}
pub fn body(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    let bag = r.eax;
    let tag = r.edx & 255;
    let target = r.ebx & 65535;
    match name {
        "EnginGetBag" => r.eax = crate::player::unit(m, r.eax)?.wrapping_add(10),
        "BagGetNbContenu" => r.eax = lo(r.eax, m.w(bag)?),
        "BagGetCode" => {
            r.edx &= 65535;
            r.eax = (r.eax & !255) | m.b(bag.wrapping_add(0x42).wrapping_add(r.edx))?
        }
        "BagGetType" => {
            r.edx &= 65535;
            r.eax = lo(r.eax, m.w(bag.wrapping_add(6).wrapping_add(2 * r.edx))?)
        }
        "BagAccepte" => {
            let amount = if (1..=4).contains(&tag) {
                1
            } else {
                r.ebx & 65535
            };
            r.edx = m.w(bag.wrapping_add(0x66))?.wrapping_add(amount);
            let cap = m.w(bag.wrapping_add(2))?;
            r.eax = (cap & !255) | u32::from((r.edx as i32) <= (cap as i32))
        }
        "BagContient" => {
            let mut n = 0u32;
            let mut i = 0u32;
            while i & 65535 < m.w(bag.wrapping_add(2))? {
                let j = i & 65535;
                if m.b(bag.wrapping_add(j).wrapping_add(0x42))? == tag {
                    let case = tag.wrapping_sub(1) & 255;
                    if matches!(case, 0 | 1 | 2 | 5 | 6) {
                        let value = m.w(bag.wrapping_add(2 * j).wrapping_add(6))?;
                        if case == 0 {
                            if m.b(m.d(0x4e19a0)?.wrapping_add(value * 276).wrapping_add(1))?
                                == target
                            {
                                n = n.wrapping_add(1)
                            }
                        } else if value == target {
                            if case == 1 {
                                n = n.wrapping_add(1)
                            } else {
                                r.eax = 1;
                                r.ebx = n;
                                r.edx = i;
                                return Ok(r);
                            }
                        }
                    }
                }
                i = i.wrapping_add(1)
            }
            r.eax = n;
            r.ebx = n;
            r.edx = i
        }
        "BagContientTroupe" => {
            let mut eax = bag;
            let mut i = 0;
            while i < m.w(bag.wrapping_add(2))? {
                eax = m.b(bag.wrapping_add(i).wrapping_add(0x42))?;
                if eax == 1 {
                    let u = m.w(bag.wrapping_add(2 * i).wrapping_add(6))?;
                    eax = u * 276;
                    let kind = m.b(m.d(0x4e19a0)?.wrapping_add(eax).wrapping_add(1))?;
                    if !matches!(kind, 3 | 11..=17 | 25) {
                        r.eax = (eax & 0xffff0000) | 0x101;
                        return Ok(r);
                    }
                    eax = (eax & 0xffff0000) | kind;
                }
                i += 1
            }
            r.eax = eax & !255
        }
        "BagPrend" => {
            let a = bag.wrapping_add(0x66);
            m.sw(a, m.w(a)?.wrapping_add(1))?;
            let n = m.w(bag)?;
            m.sb(bag.wrapping_add(n).wrapping_add(0x42), tag)?;
            m.sw(bag.wrapping_add(2 * n).wrapping_add(6), target)?;
            m.sw(bag, n.wrapping_add(1))?;
            r.ebx = n;
            r.edx = n
        }
        "BagRetire" => {
            let mut value = 65535u32;
            let mut found = None;
            let mut ebx = 0u32;
            let mut edx = r.edx;
            let mut ecx = (r.ecx & 0xffff0000) | (r.ecx & 0xff00);
            let mut edi = r.edi;
            loop {
                if ebx & 65535 >= m.w(bag)? {
                    break;
                }
                edx = value & 65535;
                if edx != 65535 {
                    break;
                }
                edx = ebx & 65535;
                let code = m.b(bag.wrapping_add(edx).wrapping_add(0x42))?;
                ecx = (ecx & 0xffff00ff) | ((code & 255) << 8);
                if code == tag {
                    edx = bag.wrapping_add(2 * edx);
                    if code == 1 {
                        let u = m.w(edx.wrapping_add(6))?;
                        let kind = m.b(m.d(0x4e19a0)?.wrapping_add(u * 276).wrapping_add(1))?;
                        if kind == target {
                            value = lo(value, m.w(bag.wrapping_add(0x66))?).wrapping_sub(1);
                            ecx = (ecx & !255) | 1;
                            m.sw(bag.wrapping_add(0x66), value)?;
                            found = Some(ebx);
                            value = lo(value, m.w(edx.wrapping_add(6))?)
                        }
                        edi = 0
                    } else {
                        edi = lo(edi, m.w(edx.wrapping_add(6))?);
                        if edi & 65535 == target {
                            ecx = lo(ecx, m.w(bag.wrapping_add(0x66))?).wrapping_sub(1);
                            found = Some(ebx);
                            m.sw(bag.wrapping_add(0x66), ecx)?;
                            ecx = (ecx & !255) | 1;
                            value = lo(value, m.w(edx.wrapping_add(6))?)
                        }
                    }
                }
                ebx = ebx.wrapping_add(1)
            }
            if let Some(j) = found {
                let mut j = j & 65535;
                loop {
                    let n = m.w(bag)?;
                    edx = bag.wrapping_add(j);
                    ebx = bag.wrapping_add(2 * j);
                    if (j as i32) >= (n.wrapping_sub(1) as i32) {
                        break;
                    }
                    m.sb(edx.wrapping_add(0x42), m.b(edx.wrapping_add(0x43))?)?;
                    m.sw(ebx.wrapping_add(6), m.w(ebx.wrapping_add(8))?)?;
                    j += 1
                }
                m.sb(edx.wrapping_add(0x42), 0)?;
                m.sw(ebx.wrapping_add(6), 0)?;
                m.sw(bag, m.w(bag)?.wrapping_sub(1))?
            }
            r.eax = value;
            r.ebx = ebx;
            r.edx = edx
        }
        _ => return stop("Core Bag owner entry", 0x4471b8),
    }
    Ok(r)
}
