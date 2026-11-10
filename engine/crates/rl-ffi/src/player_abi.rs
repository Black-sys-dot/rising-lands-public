//! ABI 1.5 owned declared input and copied diagnostic event stream.
use super::*;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlInputMessage {
    pub message: u32,
    pub wparam: u32,
    pub lparam: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlInputKey {
    pub vk: u32,
    pub value: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlDeclaredInput {
    pub size: u32,
    pub flags: u32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub messages: *const RlInputMessage,
    pub message_count: u32,
    pub keys: *const RlInputKey,
    pub key_count: u32,
}
/// # Safety
/// All input spans are readable and disjoint from the writable input/output.
/// Values are copied before execution. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn rl_passo_quadro_entrada(
    handle: u64,
    phase: u32,
    input: *mut rl_core::FrameInput,
    declared: *const RlDeclaredInput,
) -> i32 {
    status(|| {
        if input.is_null() || declared.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        let d = unsafe { declared.read() };
        if d.size != std::mem::size_of::<RlDeclaredInput>() as u32
            || d.flags & !7 != 0
            || !matches!(phase, 2 | 3)
            || d.message_count > 65536
            || d.key_count > 256
            || (d.message_count != 0 && d.messages.is_null())
            || (d.key_count != 0 && d.keys.is_null())
            || (phase == 3 && d.message_count != 0)
        {
            return Err(AbiError::InvalidArgument);
        }
        let cursor = if d.flags & 1 != 0 {
            if !(0..800).contains(&d.cursor_x) || !(0..600).contains(&d.cursor_y) {
                return Err(AbiError::InvalidArgument);
            }
            Some((d.cursor_x, d.cursor_y))
        } else {
            None
        };
        let mut answers = rl_core::player::DeclaredInput {
            cursor,
            display_text_boundary: d.flags & 2 != 0,
            display_sound_boundary: d.flags & 4 != 0,
            ..Default::default()
        };
        if d.message_count != 0 {
            for m in unsafe { slice::from_raw_parts(d.messages, d.message_count as usize) } {
                if !matches!(m.message, 0x100 | 0x101 | 0x200..=0x205) {
                    return Err(AbiError::InvalidArgument);
                }
                answers.messages.push([m.message, m.wparam, m.lparam]);
            }
        }
        if d.key_count != 0 {
            for k in unsafe { slice::from_raw_parts(d.keys, d.key_count as usize) } {
                if k.vk > 255 || answers.keys.insert(k.vk, k.value as u32).is_some() {
                    return Err(AbiError::InvalidArgument);
                }
            }
        }
        let value = unsafe { input.read() };
        let out = with_world(handle, |w| Ok(w.input_step(phase, value, answers)?))?;
        unsafe { input.write(out) };
        Ok(())
    })
}
/// # Safety
/// Required is writable; buffer is writable for capacity records. Querying
/// preserves the most recent input attempt, including diagnostics after a stop.
#[no_mangle]
pub unsafe extern "C" fn rl_eventos_entrada(
    handle: u64,
    buffer: *mut rl_core::player::PlayerEvent,
    capacity: u32,
    required: *mut u32,
) -> i32 {
    status(|| {
        if required.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        with_world(handle, |w| {
            let n = u32::try_from(w.player_events.len()).map_err(|_| AbiError::ResourceLimit)?;
            unsafe { required.write(n) };
            if capacity < n {
                return Err(AbiError::BufferTooSmall);
            }
            if n != 0 && buffer.is_null() {
                return Err(AbiError::InvalidArgument);
            }
            if n != 0 {
                unsafe { ptr::copy_nonoverlapping(w.player_events.as_ptr(), buffer, n as usize) }
            }
            Ok(())
        })
    })
}
