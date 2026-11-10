//! Focused coupling for a type-0 attack phase retained by state19.
//! The body, sprite binding and effects already live in unit_display_extra.
use crate::{render::Renderer, unit_display_extra, FrameRegisters, MemoriaJogo};

/// Conservative adoption domain, not an assertion that native rejects other
/// byte values. EnginSetTypeCaract gives type0 move8/attack10; state19 can retain
/// either last attack phase. Native4159f1 ->41596d ->412390 does not reset it.
/// Leave count-valid frames and every other family with their existing caller.
pub fn supports(r: &[u8]) -> bool {
    r.len() == 276
        && r[0] == 0
        && r[1] == 0
        && r[2] == 19
        // Ordinary render does not yet publish the jitter permutation cursor.
        // Keep that independent runtime obligation outside this coupling.
        && r[0x99] == 0
        && (1..=16).contains(&r[0x91])
        && r[0xa5] == 8
        && r[0xa6] == 10
        && (r[0xa5]..r[0xa6]).contains(&r[0xa4])
        && unit_display_extra::supports(r)
}

/// Same void-effects contract as the existing provider: signed low-word EAX/EDX,
/// low-word EBX index, ordered prefix observations/writes, no scratch GPR result.
/// Scene visibility/order and publication remain with the ordinary renderer.
pub fn display(
    renderer: &mut Renderer,
    m: &MemoriaJogo,
    r: FrameRegisters,
) -> crate::render::Result<unit_display_extra::Effects> {
    let record = m.read(m.u32(0x4e19a0)?.wrapping_add(276 * (r.ebx & 65535)), 276)?;
    if !supports(record) {
        return Ok(unit_display_extra::Effects {
            writes: Vec::new(),
            observations: Vec::new(),
            stop: Some((0x412390, "animation-range adoption domain".into())),
        });
    }
    unit_display_extra::display(renderer, m, r)
}

#[cfg(test)]
mod tests {
    use super::supports;

    #[test]
    fn inherited_phases_leave_legacy_and_unknown_records_with_the_caller() {
        let mut r = [0; 276];
        r[2] = 19;
        r[0x91] = 2;
        r[0xa5] = 8;
        r[0xa6] = 10;
        for phase in [8, 9] {
            r[0xa4] = phase;
            assert!(supports(&r));
        }
        for (offset, value) in [
            (0xa4, 7),
            (0xa4, 10),
            (0x91, 0),
            (0x91, 17),
            (0xa5, 9),
            (0xa6, 11),
            (0x99, 1),
            (0, 4),
            (1, 1),
            (2, 1),
        ] {
            let mut outside = r;
            outside[offset] = value;
            assert!(!supports(&outside));
        }
        assert!(!supports(&r[..275]));
    }
}
