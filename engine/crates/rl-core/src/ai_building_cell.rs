//! I_DonneBatimentSurCase (intact 0x480ace): a word cell, no player input.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

/// Return the first existing structure whose rectangular footprint contains
/// `cell`, or 0xffffffff. The native helper ignores ownership and collision
/// masks, uses non-wrapping cell sums, and scans indices 0..1000 in order.
/// No gameplay byte, callback or RNG state is changed.
pub fn on_cell(m: &mut MemoriaJogo, cell: u16) -> Result<u32> {
    let index = crate::chain_s1::entry(
        m,
        "ChercheCaseBatiment",
        FrameRegisters {
            eax: u32::from(cell),
            ..FrameRegisters::default()
        },
    )?
    .eax & 0xffff;
    Ok(if index == 0xffff { u32::MAX } else { index })
}

/// Scheduler adapter: only the low word of the first argument is consumed.
pub fn from_arguments(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let Some(&cell) = arguments.first() else {
        return stop("I_DonneBatimentSurCase missing cell", 0x480ace);
    };
    on_cell(m, cell as u16)
}

/// Exact wrapper ABI: read WORD[EAX], preserve every non-EAX data register,
/// and pop one return address. Neither argument high word nor player is read.
pub fn entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let cell = m.u16(r.eax)?;
    r.eax = on_cell(m, cell)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_only_argument_needs_neither_high_word_player_nor_stack_bytes() {
        let mut m =
            MemoriaJogo::from_blocks([(0x494000, vec![0; 0x8f000]), (0x1200ffe, vec![0xff, 0xff])])
                .unwrap();
        let b = 0x49a0c8 + 276 * 999;
        m.set_u8(b + 5, 4).unwrap();
        m.set_u8(b + 4, 255).unwrap();
        m.set_u8(b + 8, 21).unwrap();
        m.set_u16(b + 2, 0xffff).unwrap();
        m.set_u16(0x495d8c + 52 * 21, 1).unwrap();
        m.set_u16(0x495d8e + 52 * 21, 1).unwrap();
        let before = m.clone();
        let r = FrameRegisters {
            eax: 0x1200ffe,
            ebx: 0x12345678,
            ecx: 0x87654321,
            edx: 0x80000000,
            esi: 0xabcdef01,
            edi: 0xffffffff,
            ebp: 0xa5a5a5a5,
            esp: 0xfffffffe,
        };
        let out = entry(&mut m, r).unwrap();
        assert_eq!(out.eax, 999);
        assert_eq!(out.esp, 2);
        assert_eq!(out.ebx, r.ebx);
        assert_eq!(out.ecx, r.ecx);
        assert_eq!(out.edx, r.edx);
        assert_eq!(out.esi, r.esi);
        assert_eq!(out.edi, r.edi);
        assert_eq!(out.ebp, r.ebp);
        assert_eq!(from_arguments(&mut m, &[u32::MAX]).unwrap(), 999);
        assert_eq!(m, before);
    }

    #[test]
    fn absent_and_truncated_arguments_remain_errors() {
        let mut m = MemoriaJogo::from_blocks([(0x1200fff, vec![0xff])]).unwrap();
        assert!(from_arguments(&mut m, &[]).is_err());
        assert!(entry(
            &mut m,
            FrameRegisters {
                eax: 0x1200fff,
                ..FrameRegisters::default()
            }
        )
        .is_err());
    }
}
