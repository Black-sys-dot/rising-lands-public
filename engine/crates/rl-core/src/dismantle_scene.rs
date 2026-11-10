//! Bounded admission of the empty neutral CAVE collapse draw (409421..4099a1
//! and its AffFlamme tail). The native 407dd3 body and its three
//! byte-permutation reads are unchanged.
use crate::{MemoriaJogo, Result};

pub(crate) fn admits(m: &MemoriaJogo, index: u32) -> Result<bool> {
    if index >= 1000 || m.u8(0x508041)? != 0 {
        return Ok(false);
    }
    let r = crate::player::structure(index);
    Ok(m.u8(r + 4)? == 200
        && m.u8(r + 5)? == 3
        && m.u8(r + 8)? == 3
        // Intact407dd3 proofs: MONDE01 seeds4/5 (damage104/102) and the
        // MONDE03 seed5 world with lane P's damage matrix1..101; the body and
        // AffFlamme thresholds depend only on the damage word itself.
        && m.u16(r + 6)? != 0
        && m.u16(r + 10)? == 100
        && m.u16(0x495d90 + 52 * 3)? == 100
        && m.u16(r + 12)? == 0
        && m.u16(r + 0x84)? <= 200
        && m.u8(r + 0x76)? <= 15
        && m.u8(r + 0x90)? == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_admission_keeps_the_proved_damage_and_world_boundaries() {
        let r = crate::player::structure(1);
        let maximum = 0x495d90 + 52 * 3;
        let mut m = MemoriaJogo::from_blocks(vec![
            (r, vec![0; 276]),
            (maximum, 100u16.to_le_bytes().to_vec()),
            (0x496fe4, vec![1]),
            (0x508041, vec![0]),
        ])
        .unwrap();
        m.set_u8(r + 4, 200).unwrap();
        m.set_u8(r + 5, 3).unwrap();
        m.set_u8(r + 8, 3).unwrap();
        m.set_u16(r + 10, 100).unwrap();
        m.set_u16(r + 0x84, 39).unwrap();
        for damage in [0, 100, 101, 102, 103, 104, 105, 65535] {
            m.set_u16(r + 6, damage).unwrap();
            assert_eq!(admits(&m, 1).unwrap(), damage != 0);
        }
        m.set_u16(r + 6, 104).unwrap();
        for (address, bytes) in [
            (r + 4, vec![0]),
            (r + 5, vec![5]),
            (r + 8, vec![2]),
            (r + 10, 99u16.to_le_bytes().to_vec()),
            (r + 12, 1u16.to_le_bytes().to_vec()),
            (r + 0x84, 201u16.to_le_bytes().to_vec()),
            (r + 0x76, vec![16]),
            (r + 0x90, vec![1]),
            (maximum, 101u16.to_le_bytes().to_vec()),
            (0x508041, vec![1]),
        ] {
            let mut trial = m.clone();
            trial.write(address, &bytes).unwrap();
            assert!(!admits(&trial, 1).unwrap(), "{address:x}");
        }
        assert!(!admits(&m, 1000).unwrap());
    }
}
