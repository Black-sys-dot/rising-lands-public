//! ABI 1.3. Every foreign byte input is copied; no Rust pointer escapes.
use super::*;
use rl_core::render::{Cache, Error, Observation, Renderer};
use std::sync::Arc;
pub(super) struct BoundRenderer {
    world: u64,
    renderer: Renderer,
    spare: Option<Renderer>,
}
#[repr(C)]
pub struct RlRenderBytes {
    pub data: *const u8,
    pub length: u32,
}
#[repr(C)]
pub struct RlRenderCache {
    pub keys: RlRenderBytes,
    pub payloads: RlRenderBytes,
    pub cursor: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlRenderResult {
    pub abi_status: i32,
    pub disposition: u32,
    pub generation: u64,
    pub frame_counter: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub required_pixels: u32,
    pub required_palette: u32,
    pub palette_generation: u64,
    pub cache_generation: u64,
    pub stop_address: u32,
    pub stop_callee: [u8; 48],
    pub stop_reason: [u8; 192],
}
fn render_error(e: Error) -> AbiError {
    match e {
        Error::Memory(e) => AbiError::Memory(e),
        Error::Invalid(s) => AbiError::RenderInput(s),
        Error::Stop(_, _) => AbiError::InvalidArgument,
    }
}
unsafe fn input<'a>(b: &RlRenderBytes, limit: usize) -> Result<&'a [u8]> {
    if b.data.is_null() || b.length == 0 || b.length as usize > limit {
        return Err(AbiError::InvalidArgument);
    }
    Ok(unsafe { slice::from_raw_parts(b.data, b.length as usize) })
}
unsafe fn cache(b: &RlRenderCache) -> Result<Cache> {
    let c = Cache {
        keys: unsafe { input(&b.keys, 1024) }?.to_vec(),
        raw: unsafe { input(&b.payloads, 270336) }?.to_vec(),
        cursor: u16::try_from(b.cursor).map_err(|_| AbiError::InvalidArgument)?,
    };
    c.validate().map_err(render_error)?;
    Ok(c)
}
fn text_into<const N: usize>(out: &mut [u8; N], s: &str) {
    let mut n = s.len().min(N - 1);
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    out[..n].copy_from_slice(&s.as_bytes()[..n]);
}
fn result(r: &Renderer, generation: u64, frame: u32) -> RlRenderResult {
    RlRenderResult {
        abi_status: 0,
        disposition: 0,
        generation,
        frame_counter: frame,
        width: 800,
        height: 600,
        stride: 800,
        required_pixels: 480000,
        required_palette: 768,
        palette_generation: 1,
        cache_generation: r.cache_generation,
        stop_address: 0,
        stop_callee: [0; 48],
        stop_reason: [0; 192],
    }
}
fn stop_callee(address: u32) -> &'static str {
    match address {
        0x41591f => "AffEngin",
        0x459454 => "AffText",
        0x414495 => "AffConstruireBatiment",
        0x4099a6 | 0x40958e => "AffBatiments",
        0x4085cb | 0x408ac8 | 0x408606 | 0x40863e | 0x40892d | 0x40f23b | 0x407e1e => {
            "AffBatiments"
        }
        0x410be4 => "AffInfoSelection",
        0x412923 => "AffDeplaceEngin",
        0x413696 => "AffExperience",
        0x410a2c => "AffContenanceEngin",
        0x4020ec => "AffJaugeBatiment",
        0x4022c8 => "BatimentPoseCadre",
        0x401a70 => "AffContenanceBatiment",
        0x417f6e => "AffCompteurMatiere",
        0x417086 => "AffZoneIconeAction",
        0x41729b => "AffTrame",
        0x4667a7 => "AffMarche",
        0x46a70c => "AffMiniMap",
        0x401d18 => "AffChrono",
        0x40f77b => "AffZoneInfo",
        0x40f8c0 => "AffInfoAide",
        0x417c7f => "AffDebug",
        0x417c8a => "EvenementAffiche",
        0x469c5f => "MessageBoxAffiche",
        0x417604 | 0x4175f2 => "AffDrapeauAlliance",
        0x40fd95 => "GetNumBob",
        0x45c72a => "AffBloc",
        0x410085 => "AffVegetTerrain",
        0x415acf => "AffVeget",
        0x41068e => "AffTerrain",
        0x416864 => "AffSprite",
        0x46ff70 => "AffProjectile",
        0x407ea7 | 0x407fb8 => "ChercheIndiceBatiment",
        0x40288b => "AffVehiculeAtelier",
        0x4135fa | 0x413650 => "EngineIconeFadiga",
        _ => "AffGame",
    }
}
/// # Safety
/// Aligned descriptors/output and readable inputs; disjoint output. Nine blobs:
/// original six banks in Graphics catalog order, PIE, shadow and light tables.
#[no_mangle]
pub unsafe extern "C" fn rl_render_criar(
    world: u64,
    assets: *const RlRenderBytes,
    count: u32,
    surface: *const RlRenderBytes,
    initial_cache: *const RlRenderCache,
    output: *mut u64,
) -> i32 {
    status(|| {
        if count != 9
            || assets.is_null()
            || surface.is_null()
            || initial_cache.is_null()
            || output.is_null()
        {
            return Err(AbiError::InvalidArgument);
        }
        let descriptors = unsafe { slice::from_raw_parts(assets, 9) };
        let blobs: Vec<_> = descriptors
            .iter()
            .map(|b| unsafe { input(b, 16 * 1024 * 1024) })
            .collect::<Result<_>>()?;
        let assets = Arc::new(rl_core::render_assets::Assets::load(&blobs).map_err(render_error)?);
        let renderer = Renderer::new(assets, unsafe { input(&*surface, 480000) }?, unsafe {
            cache(&*initial_cache)
        }?)
        .map_err(render_error)?;
        let handle = with_registry(|r| {
            if !r.worlds.contains_key(&world) {
                return Err(AbiError::InvalidHandle);
            }
            let handle = r.next;
            r.next = r.next.checked_add(1).ok_or(AbiError::ResourceLimit)?;
            r.renderers.insert(
                handle,
                BoundRenderer {
                    world,
                    renderer,
                    spare: None,
                },
            );
            Ok(handle)
        })?;
        unsafe { output.write(handle) };
        Ok(())
    })
}
/// # Safety
/// Outputs are aligned, writable and disjoint. Generation changes on state writes.
#[no_mangle]
pub unsafe extern "C" fn rl_render_geracao(
    world: u64,
    generation: *mut u64,
    phase: *mut u32,
    frame: *mut u32,
) -> i32 {
    status(|| {
        if generation.is_null() || phase.is_null() || frame.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        let (g, p, f) = with_world(world, |w| {
            Ok((
                w.generation,
                w.frame_phase,
                w.state()
                    .frame
                    .as_ref()
                    .ok_or(AbiError::InvalidArgument)?
                    .globals
                    .iteration,
            ))
        })?;
        unsafe {
            generation.write(g);
            phase.write(p);
            frame.write(f);
        }
        Ok(())
    })
}
/// # Safety
/// Caller output buffers/result are valid and disjoint. Size query writes only
/// result metadata, never pixels/palette/cache. Partial drawing is a successful
/// result with disposition=2 and named stop, never an ABI error.
/// Published result fields are defined; repr(C) alignment padding is unspecified.
/// Other errors leave the entire caller result storage untouched.
#[no_mangle]
pub unsafe extern "C" fn rl_render_quadro(
    handle: u64,
    expected_generation: u64,
    hud_flags: u32,
    pixels: *mut u8,
    pixel_capacity: u32,
    palette: *mut u8,
    palette_capacity: u32,
    output: *mut RlRenderResult,
) -> i32 {
    status(|| {
        if output.is_null() || hud_flags > 1 {
            return Err(AbiError::InvalidArgument);
        }
        with_registry(|r| {
            let bound = r
                .renderers
                .get_mut(&handle)
                .ok_or(AbiError::InvalidHandle)?;
            let world = r
                .worlds
                .get_mut(&bound.world)
                .ok_or(AbiError::InvalidHandle)?;
            if world.generation != expected_generation {
                return Err(AbiError::StaleGeneration);
            }
            if !matches!(world.frame_phase, 1 | 3) {
                return Err(AbiError::InvalidArgument);
            }
            let frame = world
                .state()
                .frame
                .as_ref()
                .ok_or(AbiError::InvalidArgument)?
                .globals
                .iteration; // checked original global via typed state
            let mut report = result(&bound.renderer, expected_generation, frame);
            if pixel_capacity < 480000 || palette_capacity < 768 {
                report.abi_status = 6;
                unsafe { output.write(report) };
                return Err(AbiError::BufferTooSmall);
            }
            if pixels.is_null() || palette.is_null() {
                return Err(AbiError::InvalidArgument);
            }
            let persistent = world.execution_mode() != 0;
            let mut next = if persistent {
                if let Some(mut spare) = bound.spare.take() {
                    spare.prepare_from(&bound.renderer);
                    spare
                } else {
                    bound.renderer.clone()
                }
            } else {
                bound.renderer.clone()
            };
            next.audit = world.execution_mode() != 1;
            let boundary = || {
                world
                    .render_boundary_memory(hud_flags == 1)
                    .map_err(|e| match e {
                        EngineError::ExecutionProjectionMismatch => AbiError::RenderInput(
                            "World render receipt does not match this boundary/flags",
                        ),
                        other => other.into(),
                    })
            };
            let outcome = if let Some((generation, flags, outcome)) = &next.last {
                if *generation == expected_generation {
                    if *flags != hud_flags {
                        return Err(AbiError::InvalidArgument);
                    }
                    outcome.clone()
                } else {
                    next.render(boundary()?.as_ref(), hud_flags == 1)
                        .map_err(render_error)?
                }
            } else {
                next.render(boundary()?.as_ref(), hud_flags == 1)
                    .map_err(render_error)?
            };
            world.publish_render_effects(&outcome)?;
            report.disposition = outcome.disposition;
            report.cache_generation = next.cache_generation;
            if let Some((address, reason)) = &outcome.stop {
                report.stop_address = *address;
                text_into(&mut report.stop_callee, stop_callee(*address));
                text_into(&mut report.stop_reason, reason);
            }
            next.last = Some((expected_generation, hud_flags, outcome));
            unsafe {
                ptr::copy_nonoverlapping(next.surface.as_ptr(), pixels, 480000);
                ptr::copy_nonoverlapping(next.assets.palette.as_ptr(), palette, 768);
                output.write(report);
            }
            let previous = std::mem::replace(&mut bound.renderer, next);
            if persistent {
                bound.spare = Some(previous);
            }
            Ok(())
        })
    })
}
/// # Safety
/// Readable aligned descriptors. Reset copies surface/cache; invalid input is atomic.
#[no_mangle]
pub unsafe extern "C" fn rl_render_reset(
    handle: u64,
    surface: *const RlRenderBytes,
    initial_cache: *const RlRenderCache,
) -> i32 {
    status(|| {
        if surface.is_null() || initial_cache.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        let surface = unsafe { input(&*surface, 480000) }?;
        let cache = unsafe { cache(&*initial_cache) }?;
        with_registry(|r| {
            let bound = r
                .renderers
                .get_mut(&handle)
                .ok_or(AbiError::InvalidHandle)?;
            if !r.worlds.contains_key(&bound.world) {
                return Err(AbiError::InvalidHandle);
            }
            bound.renderer.reset(surface, cache).map_err(render_error)
        })
    })
}
#[no_mangle]
pub extern "C" fn rl_render_liberar(handle: u64) -> i32 {
    status(|| {
        with_registry(|r| {
            r.renderers
                .remove(&handle)
                .map(|_| ())
                .ok_or(AbiError::InvalidHandle)
        })
    })
}
/// # Safety
/// Copied last-render observations; required writable/aligned, buffer valid for capacity records.
#[no_mangle]
pub unsafe extern "C" fn rl_render_observacoes(
    handle: u64,
    buffer: *mut Observation,
    capacity: u32,
    required: *mut u32,
) -> i32 {
    status(|| {
        if required.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        with_registry(|r| {
            let bound = r.renderers.get(&handle).ok_or(AbiError::InvalidHandle)?;
            if !r.worlds.contains_key(&bound.world) {
                return Err(AbiError::InvalidHandle);
            }
            let events = bound
                .renderer
                .last
                .as_ref()
                .map(|(_, _, o)| o.observations.as_slice())
                .unwrap_or(&[]);
            unsafe { required.write(events.len() as u32) };
            if capacity < events.len() as u32 {
                return Err(AbiError::BufferTooSmall);
            }
            if !events.is_empty() {
                if buffer.is_null() {
                    return Err(AbiError::InvalidArgument);
                }
                unsafe {
                    ptr::copy_nonoverlapping(events.as_ptr(), buffer, events.len());
                }
            }
            Ok(())
        })
    })
}
/// # Safety
/// Copies exactly 1024 key bytes and 270336 raw payload bytes. Capacities are bytes.
/// A query writes required sizes/cursor only; no output buffer changes.
#[no_mangle]
pub unsafe extern "C" fn rl_render_cache(
    handle: u64,
    keys: *mut u8,
    key_capacity: u32,
    payloads: *mut u8,
    payload_capacity: u32,
    key_required: *mut u32,
    payload_required: *mut u32,
    cursor: *mut u32,
) -> i32 {
    status(|| {
        if key_required.is_null() || payload_required.is_null() || cursor.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        with_registry(|r| {
            let bound = r.renderers.get(&handle).ok_or(AbiError::InvalidHandle)?;
            if !r.worlds.contains_key(&bound.world) {
                return Err(AbiError::InvalidHandle);
            }
            unsafe {
                key_required.write(1024);
                payload_required.write(270336);
                cursor.write(bound.renderer.cache.cursor as u32);
            }
            if key_capacity < 1024 || payload_capacity < 270336 {
                return Err(AbiError::BufferTooSmall);
            }
            if keys.is_null() || payloads.is_null() {
                return Err(AbiError::InvalidArgument);
            }
            unsafe {
                ptr::copy_nonoverlapping(bound.renderer.cache.keys.as_ptr(), keys, 1024);
                ptr::copy_nonoverlapping(bound.renderer.cache.raw.as_ptr(), payloads, 270336);
            }
            Ok(())
        })
    })
}

/// Graphics render contract, independent of the additive host ABI minor version.
#[no_mangle]
pub extern "C" fn rl_render_contrato() -> u32 {
    0x00010009
}
/// # Safety
/// Copied last saved-background bytes. Metadata outputs must be aligned/writable
/// and disjoint from buffer. A query never advances the renderer/cache.
#[no_mangle]
pub unsafe extern "C" fn rl_render_fundo_salvo(
    handle: u64,
    buffer: *mut u8,
    capacity: u32,
    required: *mut u32,
    address: *mut u32,
    generation: *mut u64,
) -> i32 {
    status(|| {
        if required.is_null() || address.is_null() || generation.is_null() {
            return Err(AbiError::InvalidArgument);
        }
        with_registry(|r| {
            let bound = r.renderers.get(&handle).ok_or(AbiError::InvalidHandle)?;
            if !r.worlds.contains_key(&bound.world) {
                return Err(AbiError::InvalidHandle);
            }
            let background = bound.renderer.saved_background.as_ref();
            let count = background.map_or(0, |b| b.pixels.len() as u32);
            unsafe {
                required.write(count);
                address.write(background.map_or(0, |b| b.address));
                generation.write(bound.renderer.cache_generation);
            }
            if capacity < count {
                return Err(AbiError::BufferTooSmall);
            }
            if let Some(bg) = background {
                if !bg.pixels.is_empty() {
                    if buffer.is_null() {
                        return Err(AbiError::InvalidArgument);
                    }
                    unsafe {
                        ptr::copy_nonoverlapping(bg.pixels.as_ptr(), buffer, bg.pixels.len());
                    }
                }
            }
            Ok(())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_background_query_copies_only_after_capacity_and_reset_is_explicit() {
        // Synthetic ABI allocation fixture; no original rendering behavior is inferred.
        let assets = Arc::new(rl_core::render_assets::Assets {
            sprites: vec![],
            palette: [0; 768],
            shadow: [0; 256],
            light: [0; 256],
        });
        let mut renderer = Renderer::new(
            assets,
            &vec![87; 480000],
            Cache {
                keys: vec![0; 1024],
                raw: vec![0; 270336],
                cursor: 0,
            },
        )
        .unwrap();
        renderer.saved_background = Some(rl_core::render::SavedBackground {
            address: 0x900000,
            pixels: vec![0, 5, 255, 7],
        });
        renderer.cache_generation = 42;
        let (world, handle) = with_registry(|r| {
            let world = r.next;
            let handle = world + 1;
            r.next += 2;
            r.worlds.insert(world, World::new(20, 0)?);
            r.renderers.insert(
                handle,
                BoundRenderer {
                    world,
                    renderer,
                    spare: None,
                },
            );
            Ok((world, handle))
        })
        .unwrap();
        let (mut required, mut address, mut generation) = (0, 0, 0);
        let mut bytes = [0x77; 4];
        unsafe {
            assert_eq!(
                rl_render_fundo_salvo(
                    handle,
                    bytes.as_mut_ptr(),
                    3,
                    &mut required,
                    &mut address,
                    &mut generation
                ),
                6
            );
            assert_eq!((required, address, generation), (4, 0x900000, 42));
            assert_eq!(bytes, [0x77; 4]);
            assert_eq!(
                rl_render_fundo_salvo(
                    handle,
                    bytes.as_mut_ptr(),
                    4,
                    &mut required,
                    &mut address,
                    &mut generation
                ),
                0
            );
            assert_eq!(bytes, [0, 5, 255, 7]);
        }
        assert_eq!(rl_render_contrato(), 0x00010009);
        assert_eq!(rl_render_liberar(handle), 0);
        assert_eq!(rl_liberar_mundo(world), 0);
    }
}
