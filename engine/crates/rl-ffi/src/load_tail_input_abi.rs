//! Independently versioned synchronous production load-tail extension to ABI 1.6.
use super::*;
use player_abi::RlDeclaredInput;

/// # Safety
/// Input/output and descriptor are aligned and disjoint; key span is readable.
/// All host answers are copied before execution. No pointer or Runtime is retained.
#[no_mangle]
pub unsafe extern "C" fn rl_load_tail_entrada_v1(
    handle: u64,
    input: *mut rl_core::FrameInput,
    declared: *const RlDeclaredInput,
) -> i32 {
    status(|| {
        if input.is_null() || declared.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        let descriptor = unsafe { declared.read() };
        if descriptor.size != std::mem::size_of::<RlDeclaredInput>() as u32
            || descriptor.flags & !6 != 0
            || descriptor.message_count != 0
            || descriptor.key_count > 256
            || (descriptor.key_count != 0 && descriptor.keys.is_null())
        {
            return Err(AbiError::InvalidArgument);
        }
        let mut answers = rl_core::player::DeclaredInput {
            display_text_boundary: descriptor.flags & 2 != 0,
            display_sound_boundary: descriptor.flags & 4 != 0,
            ..Default::default()
        };
        if descriptor.key_count != 0 {
            for key in
                unsafe { slice::from_raw_parts(descriptor.keys, descriptor.key_count as usize) }
            {
                if key.vk > 255 || answers.keys.insert(key.vk, key.value as u32).is_some() {
                    return Err(AbiError::InvalidArgument);
                }
            }
        }
        let value = unsafe { input.read() };
        let output = with_registry(|registry| {
            if save_abi::has_pending(registry, handle) {
                return Err(AbiError::Save(rl_core::save::SaveError::Busy));
            }
            let world = registry
                .worlds
                .get_mut(&handle)
                .ok_or(AbiError::InvalidHandle)?;
            Ok(world.load_tail_input_v1(value, answers)?)
        })?;
        unsafe { input.write(output) };
        Ok(())
    })
}
