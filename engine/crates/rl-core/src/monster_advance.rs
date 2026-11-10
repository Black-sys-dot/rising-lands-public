//! AvanceMonstre's mission 0x17 branch, intact 0x47b2d3..0x47b319.
//! The caller owns first-call timestamps and mission dispatch. No RNG is drawn.
use crate::{frame_tasks, MemoriaJogo, Result};

#[cfg(feature = "validation-trace")]
std::thread_local! {
    static CALLBACKS: std::cell::RefCell<Option<Vec<[u32; 9]>>> = const {
        std::cell::RefCell::new(None)
    };
}

/// Enable fixture-only callback observations; no callback can be substituted.
#[cfg(feature = "validation-trace")]
pub fn start_validation_trace() {
    CALLBACKS.with(|t| *t.borrow_mut() = Some(Vec::new()));
}

#[cfg(feature = "validation-trace")]
pub fn take_validation_trace() -> Vec<[u32; 9]> {
    CALLBACKS.with(|t| t.borrow_mut().take().unwrap_or_default())
}

#[cfg(feature = "validation-trace")]
fn observe(event: [u32; 9]) {
    CALLBACKS.with(|t| {
        if let Some(events) = t.borrow_mut().as_mut() {
            events.push(event);
        }
    });
}

/// Void effects API. The task allocator/list provider executes normally,
/// including errors and partial stores. Pending changes only on successful post.
pub fn pending(m: &mut MemoriaJogo) -> Result<()> {
    let count = m.u8(0x50b134)?;
    if count != 0 {
        let group = m.u32(0x50b134u32.wrapping_add(u32::from(count).wrapping_mul(4)))?;
        // EAX/EDX/EBX/ECX then four stack arguments, iaProbPost 0x47d920.
        let post = [4, 0x12, group, 0, 0, 0, 0, 0];
        #[cfg(feature = "validation-trace")]
        observe([0, 4, 0x12, group, 0, 0, 0, 0, 0]);
        let result = frame_tasks::post(m, &post)?;
        #[cfg(feature = "validation-trace")]
        observe([1, result, 0, 0, 0, 0, 0, 0, 0]);
        if result != u32::MAX {
            // Reload: DEC operates on memory after the callback returns.
            m.set_u8(0x50b134, m.u8(0x50b134)?.wrapping_sub(1))?;
        }
    }
    Ok(())
}
