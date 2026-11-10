//! Notification transport for the retained actual AI combat callers.
//!
//! Reuse MessageBoxSet46994b and the existing original EVENTS.INI input. This
//! module neither creates replacement text nor installs host state implicitly.
//! Registration and caller routing are supplied as an UNAPPLIED adoption patch.
use crate::{EngineError, FrameRegisters, MemoriaJogo, Result};

/// Test observation only. All declared host shortcuts stay false; the ordinary
/// sound provider still executes and validates the original unopened image.
#[cfg(feature = "validation-trace")]
pub fn observe_requests(m: &mut MemoriaJogo) {
    m.player = Some(crate::player::Runtime::default());
}

#[cfg(feature = "validation-trace")]
pub fn sound_requests(m: &MemoriaJogo) -> Vec<[u32; 7]> {
    m.player
        .as_ref()
        .into_iter()
        .flat_map(|runtime| runtime.events.iter())
        .filter(|event| event.kind == 10)
        .map(|event| event.values[..7].try_into().expect("seven sound words"))
        .collect()
}

/// AL=kind, DX=profile ID; void effects, including partial stores on failure.
/// The producer requests EVENEMENT before TEXTE, splits the first '*', sets
/// lifetime400, then advances tail/full. It makes no RNG or sound-device call.
pub fn enqueue(m: &mut MemoriaJogo, kind: u8, code: u16) -> Result<()> {
    crate::message_box::set(m, kind, code)
}

/// Existing MessageBoxCalcul469dce effects, including u16 timer underflow.
pub fn calculate(m: &mut MemoriaJogo) -> Result<()> {
    crate::frame_ai::call(m, "MessageBoxCalcul", &[])?;
    Ok(())
}

/// Retained PlayerResetAllie46d8cb owner bounds/store/notification order.
/// PlayerValideAlliance 0x46d927 (AL = a, DL = b): for (a, b) then (b, a),
/// when the two differ and both are <= 4, set bit 1 << second in the first
/// player's alliance byte +6 (player table 0x5080a8, stride 20) and, for the
/// local first player, MessageBoxSet(10, second + 0x18f); then MAJSympathie
/// (a, b, 0x10) and (b, a, 0x10), whatever the players.
pub fn valide_alliance(m: &mut MemoriaJogo, a: u32, b: u32) -> Result<()> {
    let (a, b) = (a & 255, b & 255);
    for (x, y) in [(a, b), (b, a)] {
        if x != y && x <= 4 && y <= 4 {
            let at = m.u32(0x5080a8)?.wrapping_add(20 * x + 6);
            m.set_u8(at, m.u8(at)? | (1 << y) as u8)?;
            if x == u32::from(m.u8(0x508041)?) {
                enqueue(m, 10, y as u16 + 0x18f)?;
            }
        }
    }
    crate::frame_economy::call(m, "MAJSympathie", &[a, b, 0x10])?;
    crate::frame_economy::call(m, "MAJSympathie", &[b, a, 0x10])?;
    Ok(())
}
pub fn reset_allie(m: &mut MemoriaJogo, owner: u8, other: u8) -> Result<()> {
    crate::message_box::reset_allie(m, owner, other)
}

/// Full MessageBoxSet register API; the other entries explicitly expose only
/// their established effects/return API. A failing call has no return-register
/// claim. The original setter preserves EBX/ECX/ESI/EDI/EBP, changes EAX/EDX,
/// and returns ESP+4. A full queue returns the input EDX in EAX unchanged.
pub fn entry(m: &mut MemoriaJogo, name: &str, r: FrameRegisters) -> Result<FrameRegisters> {
    let mut out = r;
    match name {
        "MessageBoxSet" => {
            let full = m.u8(0x507d58)? != 0;
            // The full-queue branch must not read the tail or profile input.
            let tail = if full { 0 } else { m.u16(0x507d56)? };
            enqueue(m, r.eax as u8, r.edx as u16)?;
            if full {
                out.eax = r.edx;
            } else {
                let next = u32::from(tail) + 1;
                out.eax = next / 256;
                out.edx = next % 256;
            }
        }
        "MessageBoxSet:effects" => enqueue(m, r.eax as u8, r.edx as u16)?,
        "PlayerResetAllie:effects" => reset_allie(m, r.eax as u8, r.edx as u8)?,
        "MessageBoxCalcul:effects" => calculate(m)?,
        "MessageBoxExiste" => {
            out.eax = crate::frame_core::call(m, "MessageBoxExiste", &[r.eax])?;
        }
        _ => return Err(EngineError::UnknownRoutine(0x46994b)),
    }
    out.esp = r.esp.wrapping_add(4);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_queue_requires_no_tail_or_profile_and_preserves_native_return() {
        let mut m = MemoriaJogo::from_blocks([(0x507d58, vec![0xff])]).unwrap();
        let before = m.clone();
        let r = FrameRegisters {
            eax: 0x1234560a,
            ebx: 0x11223344,
            ecx: 0xaabbccdd,
            edx: 0xdeadbeef,
            esi: 3,
            edi: 4,
            ebp: 5,
            esp: 0xffff_fffc,
        };
        let out = entry(&mut m, "MessageBoxSet", r).unwrap();
        assert_eq!(out.eax, r.edx);
        assert_eq!(out.edx, r.edx);
        assert_eq!(out.ebx, r.ebx);
        assert_eq!(out.ecx, r.ecx);
        assert_eq!(out.esi, r.esi);
        assert_eq!(out.edi, r.edi);
        assert_eq!(out.ebp, r.ebp);
        assert_eq!(out.esp, 0);
        assert_eq!(m, before);
    }

    #[test]
    fn expiry_wraps_head_and_zero_timer_underflows_instead_of_expiring() {
        let mut m = MemoriaJogo::from_blocks([
            (0x507d54, vec![255, 0, 0, 0, 1]),
            (0x4e7858 + 518 * 255, vec![1, 0]),
        ])
        .unwrap();
        calculate(&mut m).unwrap();
        assert_eq!(m.u16(0x507d54).unwrap(), 0);
        assert_eq!(m.u8(0x507d58).unwrap(), 0);
        let mut m =
            MemoriaJogo::from_blocks([(0x507d54, vec![0, 0, 1, 0, 0]), (0x4e7858, vec![0, 0])])
                .unwrap();
        calculate(&mut m).unwrap();
        assert_eq!(m.u16(0x4e7858).unwrap(), 65535);
        assert_eq!(m.u16(0x507d54).unwrap(), 0);
    }
}
