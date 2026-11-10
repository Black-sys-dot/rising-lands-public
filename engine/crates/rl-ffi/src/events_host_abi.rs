//! Additive ABI: directory and bytes live only for this call and are copied.
use super::*;
#[repr(C)]
pub struct RlEventsHostV1 {
    pub size: u32,
    pub version: u32,
    pub directory: *const u8,
    pub directory_length: u32,
    pub events: *const u8,
    pub events_length: u32,
}
pub(super) unsafe fn input<'a>(
    host: *const RlEventsHostV1,
) -> Result<rl_core::events_host::Input<'a>> {
    if host.is_null() {
        return Err(AbiError::InvalidArgument);
    }
    let h = unsafe { &*host };
    if h.size as usize != std::mem::size_of::<RlEventsHostV1>() || h.version != 1 {
        return Err(AbiError::VersionMismatch);
    }
    if h.directory.is_null()
        || h.events.is_null()
        || h.directory_length > 244
        || h.directory_length == 0
        || h.events_length == 0
        || h.events_length > 1024 * 1024
    {
        return Err(AbiError::InvalidArgument);
    }
    let value = rl_core::events_host::Input {
        directory: unsafe { slice::from_raw_parts(h.directory, h.directory_length as usize) },
        file: unsafe { slice::from_raw_parts(h.events, h.events_length as usize) },
    };
    value.validate()?;
    Ok(value)
}
/// # Safety
/// Aligned descriptor and readable, disjoint spans, valid for this call only.
#[no_mangle]
pub unsafe extern "C" fn rl_eventos_reassociar_v1(handle: u64, host: *const RlEventsHostV1) -> i32 {
    status(|| {
        let h = unsafe { input(host) }?;
        with_world(handle, |w| Ok(h.verify(&w.project()?)?))
    })
}
