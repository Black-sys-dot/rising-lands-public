//! Additive ABI: the original in-game FLC events (motor/include/event-flc.md).
use super::*;
/// One `Events\NAME.FLC` of the CD root in use. `available` 0: the file does not
/// open (original FlicOpen returns 0); 1: `header` holds its first 128 bytes.
#[repr(C)]
pub struct RlEventFlcV1 {
    pub name: [u8; 16],
    pub available: u32,
    pub header: [u8; 128],
}
#[repr(C)]
#[derive(Default)]
pub struct RlEventFlcStateV1 {
    pub running: u32,
    pub modal: u32,
    pub index: u32,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
}
/// # Safety
/// `entries` is readable for `count` aligned records for this call only.
#[no_mangle]
pub unsafe extern "C" fn rl_eventos_flc_v1(
    handle: u64,
    entries: *const RlEventFlcV1,
    count: u32,
) -> i32 {
    status(|| {
        if entries.is_null() || count == 0 || count > 256 {
            return Err(AbiError::InvalidArgument);
        }
        let records = unsafe { slice::from_raw_parts(entries, count as usize) };
        let mut declared = Vec::with_capacity(records.len());
        for r in records {
            let len = r.name.iter().position(|&c| c == 0).unwrap_or(16);
            if len == 16 || r.available > 1 {
                return Err(AbiError::InvalidArgument);
            }
            declared.push(rl_core::event_flc::Declared {
                name: &r.name[..len],
                header: (r.available == 1).then_some(&r.header[..]),
            });
        }
        with_world(handle, |world| Ok(world.load_event_flc(&declared)?))
    })
}
/// # Safety
/// `output` is aligned and writable; unchanged on failure.
#[no_mangle]
pub unsafe extern "C" fn rl_evento_flc_estado_v1(
    handle: u64,
    output: *mut RlEventFlcStateV1,
) -> i32 {
    status(|| {
        if output.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        let s = with_world(handle, |world| Ok(world.event_state()?))?;
        unsafe {
            output.write(RlEventFlcStateV1 {
                running: u32::from(s.running),
                modal: u32::from(s.modal),
                index: s.index,
                frames: u32::from(s.frames),
                width: u32::from(s.width),
                height: u32::from(s.height),
                speed: s.speed,
            })
        };
        Ok(())
    })
}
#[no_mangle]
pub extern "C" fn rl_evento_flc_terminar_v1(handle: u64) -> i32 {
    status(|| with_world(handle, |world| Ok(world.event_finish()?)))
}
