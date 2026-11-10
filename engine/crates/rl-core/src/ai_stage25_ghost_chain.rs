//! Native MONDE25 RULE continuation after B_ReserveGhostBatiment.
//! Direct intact assembly port; see research/ai-stage25-ghost-chain.md.
use crate::{frame::stop, FrameRegisters as R, MemoriaJogo, Result};

/// GetTypeBatimentCapacite 0x4892a4. The type is a full DWORD, with wrapping
/// table arithmetic. Only types 14 and 19 subtract reserved population.
pub fn capacity(m: &MemoriaJogo, kind: u32) -> Result<u32> {
    let n = u32::from(m.u8(0x495d9eu32.wrapping_add(kind.wrapping_mul(52)))?);
    Ok(match kind {
        14 => n.wrapping_sub(1),
        19 => n.wrapping_sub(u32::from(m.u8(0x495be8)?)),
        _ => n,
    })
}

/// B_TestBatimentBesoinColon 0x480abd tests nonzero, including underflow.
pub fn needs_colonist(m: &MemoriaJogo, kind: u32) -> Result<u32> {
    Ok(if capacity(m, kind)? != 0 { 1 } else { u32::MAX })
}

/// I_DeterminationEnnemi 0x480d68. Native rectangular search ignores mask,
/// ownership and HP. Reuse its existing provider, then the building-owner leaf.
pub fn enemy(m: &mut MemoriaJogo, cell: u16) -> Result<u32> {
    let index = crate::ai_building_cell::on_cell(m, cell)?;
    if index == u32::MAX {
        Ok(u32::MAX)
    } else {
        Ok(u32::from(m.u8(crate::player::structure(index) + 4)?))
    }
}

fn difference(a: u32, b: u32) -> u32 {
    if (a as i32) < (b as i32) {
        b.wrapping_sub(a)
    } else {
        a.wrapping_sub(b)
    }
}

/// B_ChoixPlaceLibre 0x480fc7. Signed comparisons precede unsigned division.
/// The ally branch intentionally returns AL=1 for ratio>10, EAX=ffffffff
/// otherwise; the caller's signed RULE interpretation must remain unchanged.
pub fn choose(m: &mut MemoriaJogo, a: [u32; 4], player: u32) -> Result<u32> {
    let denominator = difference(a[0], a[1]);
    let numerator = difference(a[0], a[2]);
    let ratio = numerator.checked_div(denominator).unwrap_or(0);
    // Preserve the real, lazy player-table read even though both original
    // branches produce the same final DWORD for this predicate.
    crate::frame_economy::call(m, "PlayerEstAllie", &[player & 255, a[3] & 255])?;
    Ok(if ratio > 10 { 1 } else { u32::MAX })
}

/// iagVilleDecBatEnCours 0x484b3c: full-width indices, WORD decrement.
pub fn decrement(m: &mut MemoriaJogo, player: u32, kind: u32) -> Result<()> {
    let p = 0x51b878u32
        .wrapping_add(player.wrapping_mul(124))
        .wrapping_add(kind.wrapping_mul(2));
    m.set_u16(p, m.u16(p)?.wrapping_sub(1))
}

/// iagVilleAbortWaitingProblems 0x484a7e, including wake callback before
/// unlink/free, with the original saved-next/previous update order.
fn abort_registers(m: &mut MemoriaJogo, mut r: R) -> Result<R> {
    let wanted = r.edx;
    let city = 0x51b84cu32.wrapping_add(r.eax.wrapping_mul(124));
    r.edx = r.eax;
    r.eax = r.eax.wrapping_mul(124);
    let mut previous = u32::MAX;
    let mut index = m.u32(city.wrapping_add(0x68))?;
    let mut visited = std::collections::BTreeSet::new();
    while index != u32::MAX {
        let offset = index.wrapping_mul(96);
        let t = 0x512320u32.wrapping_add(offset);
        let next = m.u32(t.wrapping_add(8))?;
        let flags = m.u8(t.wrapping_add(0x5d))?;
        r.eax = m.u32(t.wrapping_add(0x10))?;
        if !visited.insert((index, previous, next, flags, r.eax)) {
            return stop("iagVilleAbortWaitingProblems nonprogress", 0x484a9f);
        }
        r.edx = 0;
        if flags & 2 != 0 && r.eax == wanted {
            crate::frame_tasks::wake(m, index, u32::MAX)?;
            r.eax = index;
            r.edx = 1;
        }
        if r.edx != 0 {
            r.eax = offset;
            if index == m.u32(city.wrapping_add(0x68))? {
                r.eax = m.u32(t.wrapping_add(8))?;
                m.set_u32(city.wrapping_add(0x68), r.eax)?;
            } else {
                r.edx = previous.wrapping_mul(96);
                r.eax = m.u32(t.wrapping_add(8))?;
                m.set_u32(0x512328u32.wrapping_add(r.edx), r.eax)?;
            }
            r.eax = offset;
            m.set_u8(t.wrapping_add(0x5d), 255)?;
        }
        previous = index;
        index = next;
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// V_BatimentNestPlusEnConstructionWithAbort 0x482610. Decrement happens
/// first, then the wrapper re-reads both DWORD arguments before aborting.
pub fn abort(m: &mut MemoriaJogo, player: u32, kind: u32) -> Result<u32> {
    decrement(m, player, kind)?;
    abort_registers(
        m,
        R {
            eax: player,
            edx: kind,
            ..R::default()
        },
    )?;
    Ok(1)
}

pub fn from_arguments(m: &mut MemoriaJogo, address: u32, a: &[u32], player: u32) -> Result<u32> {
    let required = if address == 0x480fc7 { 4 } else { 1 };
    if a.len() < required {
        return stop("MONDE25 ghost chain missing arguments", address);
    }
    match address {
        0x480abd => needs_colonist(m, a[0]),
        0x480d68 => enemy(m, a[0] as u16),
        0x480fc7 => choose(m, [a[0], a[1], a[2], a[3]], player),
        0x482610 => abort(m, player, a[0]),
        _ => stop("unknown MONDE25 ghost chain RULE", address),
    }
}

/// Exact pointer/register APIs; no eager reads of unused player/operand bytes.
pub fn entry(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    r.eax = match name {
        "GetTypeBatimentCapacite" => capacity(m, r.eax)?,
        "B_TestBatimentBesoinColon" => needs_colonist(m, m.u32(r.eax)?)?,
        "I_DeterminationEnnemi" => enemy(m, m.u16(r.eax)?)?,
        "B_ChoixPlaceLibre" => {
            let a = [
                m.u32(r.eax)?,
                m.u32(r.eax.wrapping_add(4))?,
                m.u32(r.eax.wrapping_add(8))?,
                u32::from(m.u8(r.eax.wrapping_add(12))?),
            ];
            let player = u32::from(m.u8(r.eax.wrapping_add(0x1c))?);
            choose(m, a, player)?
        }
        "iagVilleDecBatEnCours" => {
            decrement(m, r.eax, r.edx)?;
            (r.eax, r.edx) = (r.edx, r.eax.wrapping_mul(124));
            r.eax
        }
        "iagVilleAbortWaitingProblems" => return abort_registers(m, r),
        "V_BatimentNestPlusEnConstructionWithAbort" => {
            let p = r.eax;
            let kind = m.u32(p)?;
            let player = m.u32(p.wrapping_add(0x1c))?;
            decrement(m, player, kind)?;
            let kind = m.u32(p)?;
            let player = m.u32(p.wrapping_add(0x1c))?;
            abort_registers(
                m,
                R {
                    eax: player,
                    edx: kind,
                    ..r
                },
            )?;
            1
        }
        _ => return stop("unknown MONDE25 ghost chain routine", 0x480abd),
    };
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
