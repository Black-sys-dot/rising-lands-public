//! Additive, stateless RL-VIS-001 surface bootstrap. No world/renderer handles.
use super::*;
use rl_core::game_hud_background::{GameHudBackground, PIXELS};

const VERSION: u32 = 1;
const ASSET_LIMIT: usize = 1024 * 1024;

#[no_mangle]
pub extern "C" fn rl_game_hud_background_version() -> u32 {
    VERSION
}

/// # Safety
/// Asset is readable for length; surface is writable for capacity. Spans must
/// be valid allocations. Errors leave every output byte unchanged. No pointers
/// are retained. Decode at entry, then retain/reuse the caller's initialized seed.
#[no_mangle]
pub unsafe extern "C" fn rl_game_hud_background_initialize(
    version: u32,
    asset: *const u8,
    length: u32,
    surface: *mut u8,
    capacity: u32,
) -> i32 {
    status(|| {
        if version != VERSION {
            return Err(AbiError::VersionMismatch);
        }
        if capacity < PIXELS as u32 {
            return Err(AbiError::BufferTooSmall);
        }
        if length as usize > ASSET_LIMIT {
            return Err(AbiError::ResourceLimit);
        }
        if asset.is_null() || surface.is_null() || length == 0 {
            return Err(AbiError::InvalidArgument);
        }
        let a = asset as usize;
        let b = a
            .checked_add(length as usize)
            .ok_or(AbiError::InvalidArgument)?;
        let c = surface as usize;
        let d = c
            .checked_add(capacity as usize)
            .ok_or(AbiError::InvalidArgument)?;
        if a < d && c < b {
            return Err(AbiError::InvalidArgument);
        }
        let background =
            GameHudBackground::load(unsafe { slice::from_raw_parts(asset, length as usize) })?;
        // The only publication follows all fallible validation/decoding. The
        // provider's exact-size check precedes its copy; capacity tail is untouched.
        background.initialize_surface(unsafe { slice::from_raw_parts_mut(surface, PIXELS) })?;
        Ok(())
    })
}

#[cfg(test)]
#[path = "game_hud_background_abi_tests.rs"]
mod tests;
