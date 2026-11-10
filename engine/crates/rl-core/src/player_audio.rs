//! Audio SoundRequestProvider's exact unopened-display domain. No device fallback.
use crate::{frame::stop, player::Memory, render_assets::sha256, MemoriaJogo, Result};
const PRIVATE_A: [u8; 32] = [
    0x0b, 0x28, 0x70, 0xb9, 0x91, 0x8d, 0x5b, 0x00, 0xab, 0x74, 0x95, 0xe5, 0x9b, 0xcd, 0x31, 0xa3,
    0x8a, 0x2e, 0x71, 0x7e, 0xe0, 0x14, 0x10, 0x6e, 0xe4, 0x3b, 0x15, 0x64, 0xc0, 0xea, 0x57, 0x06,
];
const PRIVATE_B: [u8; 32] = [
    0xb6, 0x6e, 0x75, 0x2a, 0xd0, 0xd4, 0xd3, 0xa5, 0xd2, 0x29, 0x36, 0x17, 0x80, 0xb3, 0xd1, 0x8b,
    0xb0, 0xf6, 0xb5, 0xbc, 0xc1, 0x97, 0xa8, 0xb8, 0x11, 0xce, 0xd5, 0x34, 0x69, 0x81, 0x60, 0x5d,
];
std::thread_local! {
    // Byte images already proven to hash to PRIVATE_A / PRIVATE_B. Acceptance
    // depends only on these bytes, so an equal image is accepted without
    // hashing again; any other image is hashed exactly as before.
    static ACCEPTED: std::cell::RefCell<Option<(Vec<u8>, Vec<u8>)>> =
        const { std::cell::RefCell::new(None) };
}
fn accepted(a: &[u8], b: &[u8]) -> bool {
    if ACCEPTED.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|(x, y)| x.as_slice() == a && y.as_slice() == b)
    }) {
        return true;
    }
    if sha256(a) != PRIVATE_A || sha256(b) != PRIVATE_B {
        return false;
    }
    ACCEPTED.with(|c| *c.borrow_mut() = Some((a.to_vec(), b.to_vec())));
    true
}
fn validate(m: &MemoriaJogo, entry: u32) -> Result<()> {
    let mut a = [0u8; 88];
    a.copy_from_slice(m.read(0x498f14, 88)?);
    if m.d(0x498f5c)? > 1 {
        return stop("SoundRequestProvider uncaptured dirty word", entry);
    }
    a[72..76].copy_from_slice(&[1, 0, 0, 0]);
    if !accepted(&a, m.read(0x521358, 3804)?) {
        return stop(
            "SoundRequestProvider outside accepted unopened private sound image",
            entry,
        );
    }
    Ok(())
}
pub fn manage(m: &mut MemoriaJogo) -> Result<u32> {
    validate(m, 0x48aaca)?;
    m.sd(0x498f5c, 0)?;
    Ok(1)
}
pub fn request(m: &mut MemoriaJogo, post: bool, r: [u32; 4]) -> Result<u32> {
    let entry = if post { 0x48aa79 } else { 0x48aac0 };
    validate(m, entry)?;
    let reference = if post { r[2] } else { r[0] };
    let supported = !post || matches!(r[0], 0x41 | 0x42 | 0x50);
    if supported && m.d(reference)? != u32::MAX {
        return stop(
            "SoundRequestProvider unopened display requires a free caller handle",
            entry,
        );
    }
    if !post {
        return Ok(0);
    }
    if !supported {
        return Ok(r[1]);
    }
    // reserve() -> HndSound_LoadDataInfo -> give_back(). The accepted empty
    // banks have no matching entry; the record's +0x18 is genuinely written.
    let index = m.d(0x498f68)?;
    if !(19..=70).contains(&index) {
        return stop("Waiting maximum index outside 19..70", 0x48aefa);
    }
    if index == 70 {
        return Ok(0);
    }
    m.sd(0x498f68, index + 1)?;
    m.sd(0x52137c + (index + 1) * 44 + 24, 0)?;
    m.sd(0x498f68, index)?;
    Ok(0)
}

#[cfg(test)]
mod accepted_image_tests {
    use super::*;
    #[test]
    fn cache_never_accepts_an_image_the_hash_refuses() {
        ACCEPTED.with(|c| *c.borrow_mut() = None);
        let a = [7u8; 88];
        let b = vec![9u8; 3804];
        assert!(!accepted(&a, &b));
        assert!(ACCEPTED.with(|c| c.borrow().is_none()));
        // A cached accepted image is matched byte for byte only.
        ACCEPTED.with(|c| *c.borrow_mut() = Some((a.to_vec(), b.clone())));
        assert!(accepted(&a, &b));
        let mut changed = b.clone();
        changed[3803] ^= 1;
        assert!(!accepted(&a, &changed));
        let mut changed_a = a;
        changed_a[0] ^= 1;
        assert!(!accepted(&changed_a, &b));
        ACCEPTED.with(|c| *c.borrow_mut() = None);
    }
}
