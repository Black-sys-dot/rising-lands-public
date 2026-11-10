//! Additive C ABI for the isolated stage-start difficulty adapter.
use super::{mission_abi, status, with_world, AbiError};
use rl_core::{
    difficulty::{apply, continue_victory, Preset},
    FrameInput,
};

/// Apply once after loading fresh stage inputs, before load-tail/frame0.
#[no_mangle]
pub extern "C" fn rl_aplicar_dificuldade(handle: u64, preset: u32) -> i32 {
    status(|| {
        let preset = Preset::try_from(preset).map_err(|_| AbiError::InvalidArgument)?;
        with_world(handle, |world| Ok(apply(world, preset)?))
    })
}

/// Mission continuation with stage-start policy before publication/load-tail.
/// # Safety
/// Same spans, alignment and disjoint-output requirements as rl_missao_continuar.
#[no_mangle]
pub unsafe extern "C" fn rl_missao_continuar_dificuldade(
    handle: u64,
    name: *const u8,
    name_len: u32,
    eta: *const u8,
    eta_len: u32,
    monsters: *const u8,
    monsters_len: u32,
    input: *const FrameInput,
    preset: u32,
    output: *mut u64,
    input_out: *mut FrameInput,
) -> i32 {
    status(|| {
        let preset = Preset::try_from(preset).map_err(|_| AbiError::InvalidArgument)?;
        if input.is_null() || output.is_null() || input_out.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        let name = unsafe { mission_abi::bytes(name, name_len) }?;
        let eta = unsafe { mission_abi::bytes(eta, eta_len) }?;
        let monsters = unsafe { mission_abi::bytes(monsters, monsters_len) }?;
        let (world, next_input) = with_world(handle, |world| {
            Ok(continue_victory(
                world,
                name,
                eta,
                monsters,
                unsafe { input.read() },
                preset,
            )?)
        })?;
        let new_handle = mission_abi::publish(world)?;
        unsafe {
            output.write(new_handle);
            input_out.write(next_input);
        }
        Ok(())
    })
}
