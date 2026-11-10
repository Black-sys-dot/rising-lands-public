//! iagMessTestSendMessageEngin descriptor continuation, intact 0x483f5d..0x484134.
//! The caller owns the existing 0x483ed7..0x483f5d prefix and its cache stores.
use crate::{EngineError, MemoriaJogo, Result};

/// Pending iagVilleSendMessage: AX=structure, DL=message, EBX=0, ECX=target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CityMessage {
    pub structure: u16,
    pub message: u8,
    pub target: u32,
}

fn unit_record(m: &MemoriaJogo, unit: u16) -> Result<u32> {
    Ok(m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(unit)))
}

fn group(index: u32) -> u32 {
    0x51a9e0u32.wrapping_add(index.wrapping_shl(5))
}

// Both descriptor operands use zero-extended offsets and widths. Unsupported
// positive widths produce zero in the original; zero offset/width retains the
// previous local value. None preserves the uninitialized native target local.
fn operand(m: &MemoriaJogo, row: u32, r: u32) -> Result<Option<u32>> {
    let width = m.u8(row)?;
    let offset = m.u16(row.wrapping_add(1))?;
    if offset == 0 || width == 0 {
        return Ok(None);
    }
    let address = r.wrapping_add(u32::from(offset));
    Ok(Some(match width {
        1 => u32::from(m.u8(address)?),
        2 => u32::from(m.u16(address)?),
        4 => m.u32(address)?,
        _ => 0,
    }))
}

/// Synchronous original unit-message dispatch, with the genuine message41 leaf.
/// Other messages and behaviour callbacks remain owned by frame_ai.
fn send_unit(m: &mut MemoriaJogo, unit: u16, message: u8) -> Result<()> {
    match crate::frame_ai::call(
        m,
        "iagUnitSendMessage",
        &[u32::from(unit), u32::from(message), 0],
    ) {
        Ok(_) => Ok(()),
        Err(EngineError::UnportedCallee {
            name: "iagUnitSendMessage message handler",
            address: 0x482a49,
        }) if message == 0x41 => {
            // Original 0x482a51..0x482ae0. The existing dispatch has already
            // checked PlayerGetType and the message interval, without stores.
            let r = unit_record(m, unit)?;
            let old = m.u32(r.wrapping_add(0x10c))?;
            if old == u32::MAX || group(old) == 0 {
                return Ok(());
            }
            crate::frame_raid::remove_unit(m, old, unit)?;
            let owner = u32::from(m.u8(r)?);
            let destination = m.u32(0x51b68c + 96 * owner)?;
            crate::frame_raid::add_unit(m, destination, unit)?;
            Ok(())
        }
        Err(error) => Err(error),
    }
}

/// Continue exactly at 0x483f5d after the existing provider has cached state and
/// mode and verified owner<4, PlayerGetType==2 and group!=-1. No cache or owner
/// check is repeated here. Original bytecode and live group/task memory are
/// required; sparse/unmapped reads remain errors, never implicit zeroes.
///
/// Effects-only API: no native scratch EAX/flags/stack locals are promised.
/// Native EBX/ECX/EDX/ESI/EDI/EBP survive the full caller; RET consumes four
/// bytes. Callback effects run wake -> unit -> city, synchronously. An unknown
/// city callback stops at its actual callee entry after all preceding stores.
pub fn continue_descriptor(m: &mut MemoriaJogo, unit: u16) -> Result<()> {
    continue_descriptor_with_city(m, unit, |_, _| {
        Err(EngineError::UnportedCallee {
            name: "iagVilleSendMessage descriptor callback",
            address: 0x484808,
        })
    })
}

/// Same continuation, with a synchronous owner for the actual city call. The
/// owner must execute that consumer or return its dependency; this layer never
/// treats an unimplemented callback as a successful message. The default entry
/// above refuses it. The fixture records its arguments and refuses it as well.
pub fn continue_descriptor_with_city(
    m: &mut MemoriaJogo,
    unit: u16,
    send_city: impl FnOnce(&mut MemoriaJogo, CityMessage) -> Result<()>,
) -> Result<()> {
    let r = unit_record(m, unit)?;
    let g = group(m.u32(r.wrapping_add(0x10c))?);
    if g == 0 {
        return Ok(());
    }
    let flags = m.u32(g.wrapping_add(0x1c))?;
    if flags == 0 {
        return Ok(());
    }
    let slot = m.u16(g.wrapping_add(0x16))?;
    if slot == u16::MAX {
        return Ok(());
    }
    let mut row = m.u32(0x498b38 + 4 * u32::from(slot))?;
    if row == 0 {
        return Ok(());
    }
    let mut mask = 0u32;
    let mut result = 1;
    let mut unit_message = 0;
    let mut city_message = 0;
    let mut target = None;
    loop {
        let kind = m.u8(row)?;
        if kind == 0xff {
            break;
        }
        if kind == m.u8(r.wrapping_add(1))?
            && m.u8(row.wrapping_add(1))? == m.u8(r.wrapping_add(2))?
        {
            let mode = m.u8(row.wrapping_add(2))?;
            if mode == 0xff || mode == m.u8(r.wrapping_add(0x89))? {
                mask |= 1u32.wrapping_shl(u32::from(m.u8(row.wrapping_add(4))?));
                if let Some(value) = operand(m, row.wrapping_add(5), r)? {
                    result = value;
                }
                unit_message = m.u8(row.wrapping_add(3))?;
                city_message = m.u8(row.wrapping_add(8))?;
                if let Some(value) = operand(m, row.wrapping_add(9), r)? {
                    target = Some(value);
                }
            }
        }
        row = row.wrapping_add(12);
    }
    if mask == 0 {
        return Ok(());
    }
    // The original reloads group membership before clearing its pending flags.
    let g = group(m.u32(r.wrapping_add(0x10c))?);
    if g == 0 {
        return Ok(());
    }
    if city_message != 0 && city_message != 0xff && target.is_none() {
        return Err(EngineError::UnportedCallee {
            name: "iagMessTestSendMessageEngin uninitialized city target",
            address: 0x4840c2,
        });
    }
    m.set_u32(g.wrapping_add(0x1c), 0)?;
    let task = m.u32(g.wrapping_add(8))?;
    if task != u32::MAX {
        crate::frame_tasks::wake(m, task, if flags & mask != 0 { result } else { u32::MAX })?;
    }
    if unit_message != 0 && unit_message != 0xff {
        send_unit(m, unit, unit_message)?;
    }
    if city_message != 0 && city_message != 0xff {
        // iagVilleSendMessage is an independent lane. In particular descriptor4
        // must retain the real group transfer above before this dependency.
        let target = target.expect("validated before the first callback");
        return send_city(
            m,
            CityMessage {
                structure: target as u16,
                message: city_message,
                target,
            },
        );
    }
    Ok(())
}
