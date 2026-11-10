//! Original state18 transport. Ground sack relocation is distinct from stock credit.
//! Typed effects APIs: native caller scratch EAX/flags are not return values.
use crate::{
    player::{structure, unit, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
#[cfg(feature = "validation-trace")]
thread_local! {
    static VALIDATION_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CALLS: std::cell::RefCell<Vec<(u32, u32, Vec<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
}
macro_rules! validation_call {
    ($site:expr, $callee:expr, $args:expr $(,)?) => {
        #[cfg(feature = "validation-trace")]
        if validation_active() {
            let call = ($site, $callee, ($args).to_vec());
            CALLS.with(|calls| calls.borrow_mut().push(call));
        }
        #[cfg(not(feature = "validation-trace"))]
        let _ = ($site, $callee);
    };
}
#[cfg(feature = "validation-trace")]
fn validation_active() -> bool {
    VALIDATION_ACTIVE.with(std::cell::Cell::get)
}
#[cfg(feature = "validation-trace")]
pub fn begin_validation_trace() {
    VALIDATION_ACTIVE.with(|active| active.set(true));
    CALLS.with(|calls| calls.borrow_mut().clear());
    OWN_STORES.with(|stores| stores.borrow_mut().clear());
}
#[cfg(feature = "validation-trace")]
pub fn take_validation_calls() -> Vec<(u32, u32, Vec<u32>)> {
    CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
}
fn destroyed(m: &MemoriaJogo, building: u32, site: u32) -> Result<bool> {
    validation_call!(site, 0x41a70b, &[building & 65535]);
    Ok(m.b(structure(building) + 5)? == 3) // BatimentEnDestruction41a70b
}
fn owner(m: &MemoriaJogo, building: u32) -> Result<u32> {
    validation_call!(0x455d52, 0x41a08c, &[building & 65535]);
    m.b(structure(building) + 4) // BatimentGetPlayer41a08c
}
/// SacGetType476038 returnsff for anything outside42..44, not object-42.
fn sack_type(m: &MemoriaJogo, cell: u32) -> Result<u32> {
    validation_call!(0x44eb0b, 0x476038, &[cell & 65535]);
    let object = m.b(m.d(0x507d74)?.wrapping_add(cell & 65535))?;
    Ok(if (0x42..=0x44).contains(&object) {
        object - 0x42
    } else {
        255
    })
}
/// SacPrend476662: an exhausted sack returns0 and still clears its object.
fn take_sack(m: &mut MemoriaJogo, cell: u32) -> Result<u32> {
    validation_call!(0x44ec1b, 0x476662, &[cell & 65535]);
    let cell = cell & 65535;
    let quantities = m.d(0x507dac)?;
    let mut resource = 0;
    if m.b(quantities.wrapping_add(cell))? != 0 {
        m.transport_sb(
            quantities.wrapping_add(cell),
            m.b(quantities.wrapping_add(cell))?.wrapping_sub(1),
        )?;
        resource = m.b(m.d(0x507d74)?.wrapping_add(cell))?.wrapping_sub(0x42) & 255;
    }
    if m.b(m.d(0x507dac)?.wrapping_add(cell))? == 0 {
        m.transport_sb(m.d(0x507d74)?.wrapping_add(cell), 0)?;
    }
    Ok(resource)
}
#[cfg(feature = "validation-trace")]
thread_local! {
    static OWN_STORES: std::cell::RefCell<Vec<(u32, Vec<u8>)>> = const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(feature = "validation-trace")]
pub fn take_validation_stores() -> Vec<(u32, Vec<u8>)> {
    OWN_STORES.with(|stores| std::mem::take(&mut *stores.borrow_mut()))
}
trait TransportStores {
    fn transport_sb(&mut self, address: u32, value: u32) -> Result<()>;
    fn transport_sw(&mut self, address: u32, value: u32) -> Result<()>;
}
impl TransportStores for MemoriaJogo {
    fn transport_sb(&mut self, address: u32, value: u32) -> Result<()> {
        self.sb(address, value)?;
        #[cfg(feature = "validation-trace")]
        if validation_active() {
            OWN_STORES.with(|stores| stores.borrow_mut().push((address, vec![value as u8])));
        }
        Ok(())
    }
    fn transport_sw(&mut self, address: u32, value: u32) -> Result<()> {
        self.sw(address, value)?;
        #[cfg(feature = "validation-trace")]
        if validation_active() {
            OWN_STORES.with(|stores| {
                stores
                    .borrow_mut()
                    .push((address, (value as u16).to_le_bytes().to_vec()))
            });
        }
        Ok(())
    }
}
// Native inline search release has three store orders at the different joins.
fn release_inline(m: &mut MemoriaJogo, u: u32, order: u8) -> Result<()> {
    if m.w(0x496304)? == u {
        m.transport_sb(0x4e0f36, 0)?;
        if order == 1 {
            m.sd(0x4e0f3c, 0)?;
            #[cfg(feature = "validation-trace")]
            if validation_active() {
                OWN_STORES.with(|s| s.borrow_mut().push((0x4e0f3c, vec![0; 4])));
            }
        }
        m.transport_sb(0x4e0f37, 0)?;
        if order == 2 {
            m.transport_sw(0x496304, 65535)?;
        }
        if order != 1 {
            m.sd(0x4e0f3c, 0)?;
            #[cfg(feature = "validation-trace")]
            if validation_active() {
                OWN_STORES.with(|s| s.borrow_mut().push((0x4e0f3c, vec![0; 4])));
            }
        }
        if order != 2 {
            m.transport_sw(0x496304, 65535)?;
        }
    }
    Ok(())
}
fn goal(m: &mut MemoriaJogo, u: u32, cell: u32) -> Result<()> {
    goal_ordered(m, u, cell, 0)
}
fn goal_ordered(m: &mut MemoriaJogo, u: u32, cell: u32, order: u8) -> Result<()> {
    let r = unit(m, u)?;
    m.transport_sw(r + 0x7e, cell)?;
    release_inline(m, u, order)?;
    m.transport_sb(r + 0x88, 0)
}
fn walk(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    validation_call!(0x44eaf3, 0x44930c, &[u]);
    crate::frame_movement::displace(m, u)?;
    Ok(())
}
fn move_order(m: &mut MemoriaJogo, u: u32, cell: u32, site: u32) -> Result<()> {
    validation_call!(site, 0x4457d3, &[u, cell & 65535]);
    crate::frame_spawn::order(m, u, cell as u16)?;
    Ok(())
}
fn bag(m: &mut MemoriaJogo, name: &str, address: u32, resource: u32, site: u32) -> Result<u32> {
    validation_call!(
        site,
        if name == "BagContient" {
            0x4471b8
        } else {
            0x4472ea
        },
        &[address, 2, resource & 65535],
    );
    Ok(crate::chain_bags::body(
        m,
        name,
        R {
            eax: address,
            edx: 2,
            ebx: resource,
            ..Default::default()
        },
    )?
    .eax)
}
fn append_resource(m: &mut MemoriaJogo, address: u32, resource: u32) -> Result<()> {
    #[cfg(feature = "validation-trace")]
    let before = validation_active().then(|| m.clone().take_validation_trace().len());
    crate::chain_bags::body(
        m,
        "BagPrend",
        R {
            eax: address,
            edx: 2,
            ebx: resource,
            ..Default::default()
        },
    )?;
    #[cfg(feature = "validation-trace")]
    {
        if let Some(before) = before {
            let trace = m.clone().take_validation_trace();
            OWN_STORES.with(|stores| stores.borrow_mut().extend_from_slice(&trace[before..]));
        }
    }
    Ok(())
}
fn room(m: &MemoriaJogo, r: u32) -> Result<bool> {
    Ok(m.w(r + 0x70)?.wrapping_add(1) <= m.w(r + 12)?)
}
fn search(m: &MemoriaJogo, u: u32, cell: u32, radius: u32, wanted: u32, site: u32) -> Result<u32> {
    validation_call!(site, 0x477b74, &[cell & 65535, radius, wanted, wanted, u]);
    crate::chain_fatigue::free_veget(m, cell, radius, wanted, wanted, u)
}
fn timer(m: &mut MemoriaJogo, r: u32) -> Result<u32> {
    let value = m.b(r + 0x8b)?.wrapping_sub(1) & 255;
    m.transport_sb(r + 0x8b, value)?;
    Ok(value)
}
fn depot(m: &mut MemoriaJogo, u: u32, r: u32) -> Result<()> {
    match m.b(r + 0x82)? {
        2 => {
            let building = m.w(r + 0x80)?;
            m.transport_sw(r + 0xc0, building)?;
            if building == 65535 {
                move_order(m, u, m.w(r + 0x7c)?, 0x44ebbf)?;
            } else {
                validation_call!(0x44ed84, 0x41aee2, &[building]);
                let cell = crate::frame_core::building_cell(m, building)?;
                m.transport_sb(r + 0x89, 2)?;
                goal(m, u, cell)?;
            }
        }
        1 => {
            let cell = m.w(r + 0x80)?;
            m.transport_sb(r + 0x89, 2)?;
            goal(m, u, cell)?;
        }
        _ => {} // Native44edf2 returns for other destination modes.
    }
    Ok(())
}
/// Full worker transport44ea78, through native dependency boundaries for ground.
pub fn worker(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let u = u32::from(u);
    let r = unit(m, u)?;
    match m.b(r + 0x89)? {
        0 => {
            if !room(m, r)? {
                m.transport_sb(r + 0x89, 1)?;
                m.transport_sb(r + 0x8b, 10)?;
                m.transport_sb(r + 0xa4, 0)?;
            } else if m.w(r + 0x7e)? != m.w(r + 8)? {
                walk(m, u)?;
            } else if sack_type(m, m.w(r + 8)?)? == m.w(r + 0x84)? {
                m.transport_sb(r + 0x89, 1)?;
                m.transport_sb(r + 0x8b, 10)?;
                m.transport_sb(r + 0xa4, 0)?;
            } else {
                let wanted = m.b(r + 0x84)?.wrapping_add(0x42) & 255;
                let found = search(m, u, m.w(r + 0xb2)?, 5, wanted, 0x44eb53)?;
                if found == 65535 {
                    move_order(m, u, m.w(r + 0x7c)?, 0x44ebbf)?;
                } else {
                    goal(m, u, found)?;
                }
            }
        }
        1 => {
            if room(m, r)? {
                if timer(m, r)? != 0 {
                    return Ok(());
                }
                let resource = take_sack(m, m.w(r + 8)?)?;
                append_resource(m, r + 10, resource)?;
                let wanted = m.b(r + 0x84)?.wrapping_add(0x42) & 255;
                let x = m.w(r + 8)?.wrapping_add(
                    {
                        validation_call!(0x44ec76, 0x4704dc, &[]);
                        crate::frame_core::random(m)?
                    } % 5,
                );
                let y = {
                    validation_call!(0x44ec91, 0x4704dc, &[]);
                    crate::frame_core::random(m)?
                } % 5;
                let centre = x.wrapping_sub(2).wrapping_add((y << 8).wrapping_sub(0x200)) & 65535;
                let found = search(m, u, centre, 5, wanted, 0x44ecb6)?;
                // The original search executes before this second capacity test.
                if found != 65535 && room(m, r)? {
                    m.transport_sb(r + 0xa4, 0)?;
                    goal_ordered(m, u, found, 1)?;
                    m.transport_sb(r + 0x89, 0)?;
                    return Ok(());
                }
            }
            depot(m, u, r)?;
        }
        2 => match m.b(r + 0x82)? {
            2 => {
                if destroyed(m, m.w(r + 0xc0)?, 0x44ee71)? {
                    move_order(m, u, m.w(r + 0x7e)?, 0x44ebbf)?;
                } else if m.b(r + 0x86)? == 0 && m.b(r + 0x87)? == 0 && {
                    validation_call!(0x44eeb6, 0x42381f, &[m.w(r + 0xc0)?, m.w(r + 8)?]);
                    crate::chain_entry::adjacent(m, m.w(r + 0xc0)? as u16, m.w(r + 8)? as u16)?
                } {
                    m.transport_sb(r + 0x89, 3)?;
                    m.transport_sb(r + 0x8b, 10)?;
                } else {
                    walk(m, u)?;
                }
            }
            1 => {
                if m.w(r + 8)? != m.w(r + 0x7e)? {
                    return walk(m, u);
                }
                let cell = m.w(r + 8)?;
                let dest = m.w(r + 0x80)?;
                validation_call!(
                    0x44ef69,
                    0x479555,
                    &[cell & 255, cell >> 8, dest & 255, dest >> 8]
                );
                let dx = (cell & 255) as i32 - (dest & 255) as i32;
                let dy = (cell >> 8) as i32 - (dest >> 8) as i32;
                if dx * dx + dy * dy > 16 {
                    goal_ordered(m, u, dest, 1)?;
                } else {
                    // Owned ground query475763 must be adopted, not reimplemented here.
                    validation_call!(
                        0x44ef89,
                        0x475763,
                        &[cell, m.b(r + 0x84)?.wrapping_add(0x42) & 255],
                    );
                    if crate::chain_cargo::accepts(
                        m,
                        cell as u16,
                        m.b(r + 0x84)?.wrapping_add(0x42) as u8,
                    )? {
                        m.transport_sb(r + 0x89, 3)?;
                        m.transport_sb(r + 0x8b, 10)?;
                    } else {
                        // 0x44efad SacCherchePlaceLibre(+0x80, wanted, unit); a found
                        // cell becomes the goal (selection released, +0x88 = 0).
                        let wanted = m.b(r + 0x84)?.wrapping_add(0x42) as u8;
                        let s = crate::chain_cargo::find_free(m, m.w(r + 0x80)? as u16, wanted, u)?;
                        if s != 65535 {
                            goal(m, u, u32::from(s))?;
                        }
                    }
                }
            }
            _ => {} // Native44eed6 returns for other destination modes.
        },
        3 => {
            // Deliberate defensive modernization approved by Root for RL-INPUT-002:
            // the original unconditionally indexes BatimentEnDestruction with c0.
            // Cold-start ground transport has no building (FFFF), making that
            // original read out of bounds. Preserve the sentinel and execute all
            // original timer/cargo effects; only this ground/None lookup is absent.
            // Valid caches and every non-ground mode retain the exact native call.
            let cached_building = m.w(r + 0xc0)?;
            let no_ground_building = m.b(r + 0x82)? == 1 && cached_building == 65535;
            if !no_ground_building && destroyed(m, cached_building, 0x44f04a)? {
                move_order(m, u, m.w(r + 0x7e)?, 0x44f05b)?;
            }
            if timer(m, r)? != 0 {
                return Ok(());
            }
            m.transport_sb(r + 0x8b, 10)?;
            let resource = m.w(r + 0x84)?;
            if bag(m, "BagContient", r + 10, resource, 0x44f094)? & 65535 != 0 {
                match m.b(r + 0x82)? {
                    2 => {
                        validation_call!(
                            0x44f0c7,
                            0x42340f,
                            &[m.b(r + 0x84)?, 1, m.b(r)?, m.w(r + 0x80)?],
                        );
                        crate::chain_entry::credit(
                            m,
                            m.b(r + 0x84)? as u8,
                            1,
                            m.b(r)? as u8,
                            m.w(r + 0x80)? as u16,
                        )?;
                        bag(m, "BagRetire", r + 10, m.w(r + 0x84)?, 0x44f0dc)?;
                        m.transport_sb(r + 0x8b, 10)?;
                    }
                    1 => {
                        validation_call!(0x44f102, 0x4438d8, &[u, 2, m.w(r + 0x84)?]);
                        unload_resource(m, u as u16, resource as u16)?;
                        m.transport_sb(r + 0x8b, 10)?; // Native44f107 joins44eeca.
                    }
                    _ => {} // Native44f0e6 returns without removing cargo.
                }
            } else {
                let source = m.w(r + 0xb2)?;
                if source != 65535 {
                    let wanted = m.b(r + 0x84)?.wrapping_add(0x42) & 255;
                    let found = search(m, u, source, 5, wanted, 0x44f136)?;
                    if found != 65535 {
                        if found != m.w(r + 0x80)? {
                            m.transport_sb(r + 0x89, 0)?;
                            goal_ordered(m, u, found, 2)?;
                        }
                        return Ok(());
                    }
                }
                if !matches!(m.b(unit(m, u)? + 2)?, 0 | 4) {
                    m.transport_sb(unit(m, u)? + 2, 7)?;
                    release_inline(m, u, if source == 65535 { 2 } else { 0 })?;
                }
            }
        }
        _ => {} // Original44eab1 branches directly to its return epilogue.
    }
    Ok(())
}
/// EnginRetireType4438d8 mode2: drain only the requested resource to ground.
/// Other cargo tags and modes are not admitted through this typed resource API.
pub(crate) fn unload_resource(m: &mut MemoriaJogo, u: u16, resource: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    let bag_address = r + 10;
    let cell = m.w(r + 8)? as u16;
    while bag(m, "BagContient", bag_address, u32::from(resource), 0x443d71)? & 65535 != 0 {
        bag(m, "BagRetire", bag_address, u32::from(resource), 0x443d85)?;
        validation_call!(
            0x443d96,
            0x47605d,
            &[u32::from(cell), u32::from(resource) & 255]
        );
        crate::chain_cargo::deposit(m, cell, resource as u8)?;
    }
    Ok(())
}

/// CalculTransporteEngin44f2d4: native worker-vs-heavy selection.
pub fn transport(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    if !(11..=17).contains(&m.b(r + 1)?) || m.b(r + 0x82)? == 1 {
        validation_call!(0x44f337, 0x44ea78, &[u32::from(u)]);
        worker(m, u)
    } else {
        heavy(m, u)
    }
}
/// CalculTransporteEngin 0x44f88c: a vehicle (kinds 11..=17) carrying sacks,
/// any depot mode other than 1. Phase in +0x89 (0x44f2c0): go to the sack;
/// load one per +0x8b timer (search radius 10, then the depot); drive to the
/// depot; turn to orientation 9 and wait for the door (depot mode 2:
/// +0x75 = 1 until TuyauEstSortie); unload, then back to the source. A phase
/// above 4 returns (0x44f894).
fn heavy(m: &mut MemoriaJogo, unit16: u16) -> Result<()> {
    let u = u32::from(unit16);
    let r = unit(m, u)?;
    let mode = m.b(r + 0x82)?;
    let wanted = m.b(r + 0x84)?.wrapping_add(0x42) & 255;
    match m.b(r + 0x89)? {
        0 => {
            let cell = m.w(r + 8)?;
            if m.w(r + 0x7e)? != cell {
                return walk(m, u);
            }
            if sack_type(m, cell)? == m.w(r + 0x84)? {
                m.transport_sb(r + 0x89, 1)?;
                m.transport_sb(r + 0x8b, 10)?;
                return m.transport_sb(r + 0xa4, 0);
            }
            let found = search(m, u, m.w(r + 0xb2)?, 10, wanted, 0x44f3b0)?;
            if found != 65535 {
                goal(m, u, found)?;
            }
        }
        1 => {
            if room(m, r)? {
                if timer(m, r)? != 0 {
                    return Ok(());
                }
                let resource = take_sack(m, m.w(r + 8)?)?;
                append_resource(m, r + 10, resource)?;
                let x = m.w(r + 8)?.wrapping_add(crate::frame_core::random(m)? % 5);
                let y = crate::frame_core::random(m)? % 5;
                let centre = x.wrapping_sub(2).wrapping_add((y << 8).wrapping_sub(0x200)) & 65535;
                let found = search(m, u, centre, 10, wanted, 0x44f4dd)?;
                if found != 65535 {
                    m.transport_sb(r + 0xa4, 0)?;
                    goal(m, u, found)?;
                    return m.transport_sb(r + 0x89, 0);
                }
            }
            // 0x44f55d; mode 1 never reaches this routine (0x44f329).
            if mode == 2 {
                let building = m.w(r + 0x80)?;
                m.transport_sw(r + 0xc0, building)?;
                if building == 65535 {
                    return goal(m, u, m.w(r + 0x7c)?);
                }
                m.transport_sb(structure(building) + 0x74, 1)?; // 0x4237df
                let c = crate::chain_entry::central_offset(m, building)?;
                m.transport_sb(r + 0x89, 2)?;
                goal(m, u, c.wrapping_sub(0xfe) & 65535)?;
            }
        }
        2 => {
            if m.w(r + 0x7e)? != m.w(r + 8)? {
                return walk(m, u);
            }
            m.transport_sb(r + 0x89, 3)?;
            if mode == 2 {
                m.transport_sb(r + 0x90, 9)?;
            }
            m.transport_sb(r + 0x8b, 10)?;
        }
        3 => {
            if m.b(r + 0x90)? != m.b(r + 0x91)? {
                crate::frame_movement::call(m, "RotateEngin", &[u])?;
                return Ok(());
            }
            if mode != 2 {
                m.transport_sb(r + 0x89, 4)?;
                return m.transport_sb(r + 0x8b, 10);
            }
            let s = structure(m.w(r + 0xc0)?);
            m.transport_sb(s + 0x75, 1)?; // 0x41a65c
            if m.b(s + 0x76)? == 15 && m.b(s + 0x75)? != 0 {
                // TuyauEstSortie 0x420c7c
                m.transport_sb(r + 0x89, 4)?;
                m.transport_sb(r + 0x8b, 0x32)?;
            }
        }
        4 => {
            let resource = m.w(r + 0x84)?;
            if bag(m, "BagContient", r + 10, resource, 0x44f76c)? & 65535 == 0 {
                if mode == 2 {
                    let s = structure(m.w(r + 0xc0)?);
                    m.transport_sb(s + 0x75, 0)?; // 0x41a65c
                    m.transport_sb(s + 0x74, 0)?; // 0x4237ff
                }
                m.transport_sw(r + 0xc0, 65535)?;
                m.transport_sb(r + 0x89, 0)?;
                return goal_ordered(m, u, m.w(r + 0xb2)?, 2);
            }
            if timer(m, r)? != 0 {
                return Ok(());
            }
            bag(m, "BagRetire", r + 10, resource, 0x44f839)?;
            if mode == 2 {
                crate::chain_entry::credit(
                    m,
                    m.b(r + 0x84)? as u8,
                    1,
                    m.b(r)? as u8,
                    m.w(r + 0x80)? as u16,
                )?;
                m.transport_sb(r + 0x8b, 0x32)?;
            } else {
                unload_resource(m, unit16, resource as u16)?;
                m.transport_sb(r + 0x8b, 10)?;
            }
        }
        _ => {}
    }
    Ok(())
}
/// EnginNextEtape455d26..455e32, called at the original state switch position.
fn dispatch(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    validation_call!(0x455e28, 0x44f2d4, &[u32::from(u)]);
    transport(m, u)
}
pub fn state18(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    if !lost_depot(m, u, 0x455d40)? {
        return dispatch(m, u);
    }
    Ok(())
}
/// The lost-depot place-back shared by EnginNextEtape state 18 and the hunger
/// state's saved state 18 (0x454fad, same body up to 0x4550c9): with depot
/// mode 2 and a destroyed or foreign depot, the unit is put back on the map
/// when it is off it (SetEnginCollision, discovery, illumination, SetMiniMap
/// 4), sent to +0x7e and its depot cleared. Returns false when nothing applies.
pub(crate) fn lost_depot(m: &mut MemoriaJogo, u: u16, site: u32) -> Result<bool> {
    lost_depot_with(m, u, site, false)
}
/// Same place-back for the hunger state's saved state 5 (0x4550ce..0x4550c9,
/// stop 0x455107): an off-map type-7 miner first ends its mine work
/// (MineEndWork +0xb2, 0x455139) before SetEnginCollision.
pub(crate) fn lost_work_depot(m: &mut MemoriaJogo, u: u16) -> Result<bool> {
    lost_depot_with(m, u, 0x4550e8, true)
}
fn lost_depot_with(m: &mut MemoriaJogo, u: u16, site: u32, mine: bool) -> Result<bool> {
    let n = u32::from(u);
    let r = unit(m, n)?;
    if m.b(r + 0x82)? != 2 {
        return Ok(false);
    }
    let building = m.w(r + 0x80)?;
    if !destroyed(m, building, site)? && owner(m, building)? == m.b(r)? {
        return Ok(false);
    }
    let live = unit(m, n)?;
    if m.b(live + 0x72)? == 0 {
        if mine && m.b(live + 1)? == 7 {
            validation_call!(0x455139, 0x4754de, &[m.w(live + 0xb2)?]);
            crate::chain_work::mine_call(m, "MineEndWork", m.w(live + 0xb2)?)?;
        }
        let live = unit(m, n)?;
        validation_call!(0x455d8f, 0x442cb4, &[n, m.w(live + 8)?]);
        crate::frame_core::call(m, "SetEnginCollision", &[n, m.w(live + 8)?])?;
        m.transport_sb(live + 0x72, 1)?;
        let live = unit(m, n)?;
        let cell = m.w(live + 8)?;
        let p = m.b(live)?;
        if !matches!(p, 200 | 4) {
            let radius = m
                .b(live + 0xbb)?
                .wrapping_add(2)
                .wrapping_add(m.b(live + 0xf5)?)
                & 65535;
            validation_call!(0x455de3, 0x476851, &[cell, radius, p]);
            crate::player_sight::area(m, cell as u16, radius, p as u8, 0)?;
            let radius = m.b(live + 0xbb)?.wrapping_add(m.b(live + 0xf5)?) & 65535;
            validation_call!(0x455e0b, 0x476cb3, &[cell, radius, m.b(live)?]);
            crate::player_sight::area(m, cell as u16, radius, m.b(live)? as u8, 1)?;
        }
        validation_call!(0x4550a4, 0x46a388, &[m.b(live)?, 4, cell]);
        crate::player_sight::minimap(m, m.b(live)? as u8, cell as u16, 4)?;
    }
    move_order(m, n, m.w(r + 0x7e)?, 0x4550b4)?;
    m.transport_sb(r + 0x82, 0)?;
    m.transport_sw(r + 0x80, 65535)?;
    Ok(true)
}

fn adjacent_to(m: &mut MemoriaJogo, site: u32, building: u32, cell: u32) -> Result<bool> {
    validation_call!(site, 0x42381f, &[building, cell]);
    crate::chain_entry::adjacent(m, building as u16, cell as u16)
}
fn arrive(m: &mut MemoriaJogo, r: u32, phase: u32) -> Result<()> {
    m.transport_sb(r + 0x8b, 10)?;
    m.transport_sb(r + 0x89, phase)
}
fn idle_release(m: &mut MemoriaJogo, u: u32, order: u8) -> Result<()> {
    let state = m.b(unit(m, u)? + 2)?;
    if !matches!(state, 0 | 4) {
        m.transport_sb(unit(m, u)? + 2, 7)?;
        release_inline(m, u, order)?;
    }
    Ok(())
}
/// CalculDechargeCaveEngin 0x44e35c (0x44e35c..0x44ea67): the shuttle unit
/// of EnginNextEtape state 20. Phase 0 walks to the shuttle building (+0xc0)
/// while the bag has room; phase 1 loads one unit of the resource (+0x84) per
/// timer tick from the player's stock (TestRessource, BatimentEnleveMatiere,
/// inline Bag append) and, once full, heads for the depot (+0x80 building in
/// mode 2, cell in mode 1); phase 2 arrives (adjacency or ground acceptance);
/// phase 3 unloads one unit per tick (BatimentAjouteMatiere or
/// EnginRetireType) and returns to +0xb2 or idles. Direct asm port, with the
/// SacCherchePlaceLibre ground search 0x475bfe.
pub fn cave_unload(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let u = u32::from(u);
    let r = unit(m, u)?;
    match m.b(r + 0x89)? {
        0 => {
            if !room(m, r)? {
                return arrive(m, r, 1);
            }
            let shuttle = m.w(r + 0xc0)?;
            if m.w(r + 8)? != m.w(r + 0x7e)? && !adjacent_to(m, 0x44e3f7, shuttle, m.w(r + 8)?)? {
                return walk(m, u);
            }
            if adjacent_to(m, 0x44e41e, shuttle, m.w(r + 8)?)? {
                return arrive(m, r, 1);
            }
            validation_call!(0x44e44f, 0x41aee2, &[shuttle]);
            let cell = crate::frame_core::building_cell(m, shuttle)?;
            goal(m, u, cell)
        }
        1 => {
            if room(m, r)? {
                if timer(m, r)? != 0 {
                    return Ok(());
                }
                let resource = m.w(r + 0x84)?;
                let player = m.b(r)?;
                let costs = [
                    u32::from(resource == 1),
                    u32::from(resource == 2),
                    u32::from(resource == 0),
                ];
                validation_call!(0x44e54b, 0x4676e5, &[player, costs[0], costs[1], costs[2]]);
                if crate::frame_economy::call(
                    m,
                    "TestRessource",
                    &[player, costs[0], costs[1], costs[2]],
                )? == 0
                {
                    return Ok(());
                }
                validation_call!(0x44e565, 0x4235bd, &[resource & 255, 1, player]);
                crate::chain_restore::debit(m, resource as u8, 1, player as u8)?;
                let bag = r + 10;
                m.transport_sw(bag + 0x66, m.w(bag + 0x66)?.wrapping_add(1) & 65535)?;
                let index = m.w(bag)?;
                m.transport_sb(bag + index + 0x42, 2)?;
                m.transport_sw(bag + 2 * index + 6, resource)?;
                m.transport_sw(bag, index.wrapping_add(1) & 65535)?;
                m.transport_sb(r + 0x8b, 10)?;
                return Ok(());
            }
            match m.b(r + 0x82)? {
                2 => {
                    let depot = m.w(r + 0x80)?;
                    if depot == 65535 {
                        return idle_release(m, u, 2);
                    }
                    validation_call!(0x44e5d2, 0x41aee2, &[depot]);
                    let cell = crate::frame_core::building_cell(m, depot)?;
                    m.transport_sb(r + 0x89, 2)?;
                    goal(m, u, cell)
                }
                1 => {
                    m.transport_sb(r + 0x89, 2)?;
                    goal_ordered(m, u, m.w(r + 0x80)?, 2)
                }
                _ => Ok(()),
            }
        }
        2 => match m.b(r + 0x82)? {
            2 => {
                if m.b(r + 0x86)? != 0
                    || m.b(r + 0x87)? != 0
                    || !adjacent_to(m, 0x44e75f, m.w(r + 0x80)?, m.w(r + 8)?)?
                {
                    return walk(m, u);
                }
                crate::player_orders::call(m, "DeselectionEngin", [u, 0, 0, 0], 0x44e76e)?;
                m.transport_sb(r + 0x89, 3)?;
                m.transport_sb(r + 0x8b, 10)
            }
            1 => {
                let cell = m.w(r + 8)?;
                if cell != m.w(r + 0x7e)? {
                    return walk(m, u);
                }
                let dest = m.w(r + 0x80)?;
                validation_call!(
                    0x44e811,
                    0x479555,
                    &[cell & 255, cell >> 8, dest & 255, dest >> 8]
                );
                let dx = (cell & 255) as i32 - (dest & 255) as i32;
                let dy = (cell >> 8) as i32 - (dest >> 8) as i32;
                if dx * dx + dy * dy > 16 {
                    return goal(m, u, dest);
                }
                let wanted = m.b(r + 0x84)?.wrapping_add(0x42) & 255;
                validation_call!(0x44e831, 0x475763, &[cell, wanted]);
                if crate::chain_cargo::accepts(m, cell as u16, wanted as u8)? {
                    m.transport_sb(r + 0x89, 3)?;
                    return m.transport_sb(r + 0x8b, 10);
                }
                // 0x44e856 SacCherchePlaceLibre(+0x80, wanted, unit); a found cell
                // becomes the goal (selection released, +0x88 = 0).
                let s = crate::chain_cargo::find_free(m, dest as u16, wanted as u8, u)?;
                if s != 65535 {
                    goal(m, u, u32::from(s))?;
                }
                Ok(())
            }
            _ => Ok(()),
        },
        3 => {
            if timer(m, r)? != 0 {
                return Ok(());
            }
            m.transport_sb(r + 0x8b, 10)?;
            let resource = m.w(r + 0x84)?;
            if bag(m, "BagContient", r + 10, resource, 0x44e920)? & 65535 != 0 {
                return match m.b(r + 0x82)? {
                    2 => {
                        validation_call!(
                            0x44e953,
                            0x42340f,
                            &[resource & 255, 1, m.b(r)?, m.w(r + 0x80)?],
                        );
                        crate::chain_entry::credit(
                            m,
                            resource as u8,
                            1,
                            m.b(r)? as u8,
                            m.w(r + 0x80)? as u16,
                        )?;
                        bag(m, "BagRetire", r + 10, resource, 0x44e969)?;
                        Ok(())
                    }
                    1 => {
                        validation_call!(0x44e98d, 0x4438d8, &[u, 2, resource]);
                        unload_resource(m, u as u16, resource as u16)
                    }
                    _ => Ok(()),
                };
            }
            let back = m.w(r + 0xb2)?;
            if back != 65535 {
                m.transport_sb(r + 0x89, 0)?;
                return goal(m, u, back);
            }
            idle_release(m, u, 0)
        }
        _ => Ok(()), // 0x44e398: phases above 3 return.
    }
}

/// EnginNextEtape state 20 (0x455b28): shuttle units. A destroyed or foreign
/// shuttle building (+0xc0), or depot (+0x80) in mode 2, places the unit back
/// exactly as state 18 does and clears the shuttle building before the move
/// order; otherwise CalculDechargeCaveEngin 0x44e35c runs (state 18 calls
/// CalculTransporteEngin 0x44f2d4 instead; the two must not be mixed).
pub fn state20(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let n = u32::from(u);
    let r = unit(m, n)?;
    let shuttle = m.w(r + 0xc0)?;
    let mut lost = destroyed(m, shuttle, 0x455b31)? || owner(m, shuttle)? != m.b(r)?;
    if !lost && m.b(r + 0x82)? == 2 {
        let depot = m.w(r + 0x80)?;
        lost = destroyed(m, depot, 0x455c31)? || owner(m, depot)? != m.b(r)?;
    }
    if !lost {
        validation_call!(0x455d1c, 0x44e35c, &[n]);
        return cave_unload(m, u);
    }
    let live = unit(m, n)?;
    if m.b(live + 0x72)? == 0 {
        validation_call!(0x455b82, 0x442cb4, &[n, m.w(live + 8)?]);
        crate::frame_core::call(m, "SetEnginCollision", &[n, m.w(live + 8)?])?;
        m.transport_sb(live + 0x72, 1)?;
        let live = unit(m, n)?;
        let cell = m.w(live + 8)?;
        let p = m.b(live)?;
        if !matches!(p, 200 | 4) {
            let radius = m
                .b(live + 0xbb)?
                .wrapping_add(2)
                .wrapping_add(m.b(live + 0xf5)?)
                & 65535;
            validation_call!(0x455bd7, 0x476851, &[cell, radius, p]);
            crate::player_sight::area(m, cell as u16, radius, p as u8, 0)?;
            let radius = m.b(live + 0xbb)?.wrapping_add(m.b(live + 0xf5)?) & 65535;
            validation_call!(0x455bff, 0x476cb3, &[cell, radius, m.b(live)?]);
            crate::player_sight::area(m, cell as u16, radius, m.b(live)? as u8, 1)?;
        }
        validation_call!(0x454ea1, 0x46a388, &[m.b(live)?, 4, cell]);
        crate::player_sight::minimap(m, m.b(live)? as u8, cell as u16, 4)?;
    }
    m.transport_sw(r + 0xc0, 65535)?;
    move_order(m, n, m.w(r + 0x7e)?, 0x4550b4)?;
    m.transport_sb(r + 0x82, 0)?;
    m.transport_sw(r + 0x80, 65535)
}

/// Supplemental actual-sack native comparisons; excluded from production builds.
#[cfg(feature = "validation-trace")]
pub fn validation_sack_leaf(m: &mut MemoriaJogo, selector: u32, cell: u16) -> Result<u32> {
    match selector {
        1 => sack_type(m, u32::from(cell)),
        2 => take_sack(m, u32::from(cell)),
        3 => {
            let resource = m.w(unit(m, u32::from(cell))? + 0x84)? as u16;
            unload_resource(m, cell, resource)?;
            Ok(0) // Typed effects API; original scratch return register is not a contract.
        }
        4 => {
            let r = unit(m, u32::from(cell))?;
            Ok(u32::from(crate::chain_cargo::accepts(
                m,
                m.w(r + 8)? as u16,
                m.b(r + 0x84)?.wrapping_add(0x42) as u8,
            )?))
        }
        _ => panic!("invalid fixture transport selector"),
    }
}
