//! Independent spell table leaves, translated directly from intact x86.
//! No dispatcher, caller routing, technology defaults, or RNG consumption.
use crate::{EngineError, FrameRegisters, MemoriaJogo, Result};

pub const GET_PORTEE: u32 = 0x4746c9;
pub const GET_COUT_MANA: u32 = 0x4746e4;
pub const GET_COUT_APPRENTISSAGE: u32 = 0x4746fe;
pub const GET_RAYON_EFFET: u32 = 0x474734;
pub const GET_TEMPS_FIN: u32 = 0x47474f;

/// Full original register ABI. AL selects a 46-byte record. Only declared
/// memory is readable; indices beyond the five shipped spells gain no defaults.
/// Cost outputs may alias each other or the table: native store/read order is
/// intentional. Output pointers into the callee's saved stack are outside this
/// Rust API; stack save projection belongs to the native fixture adapter.
pub fn entry(m: &mut MemoriaJogo, address: u32, mut r: FrameRegisters) -> Result<FrameRegisters> {
    // The machine computes 23*AL before addressing [EAX*2 + field].
    let index = (r.eax & 255).wrapping_mul(23);
    let offset = index.wrapping_mul(2);
    match address {
        GET_PORTEE | GET_RAYON_EFFET | GET_TEMPS_FIN => {
            let field = match address {
                GET_PORTEE => 0x49717e_u32,
                GET_RAYON_EFFET => 0x497170,
                _ => 0x497172,
            };
            r.eax = (index & 0xffff0000) | u32::from(m.u16(field.wrapping_add(offset))?);
        }
        GET_COUT_MANA => {
            // MOV AL preserves AH, including for raw declared indices >=12.
            r.eax = (index & 0xffffff00) | u32::from(m.u8(0x49717c_u32.wrapping_add(offset))?);
        }
        GET_COUT_APPRENTISSAGE => {
            let stone_output = r.ebx;
            r.ebx = u32::from(m.u16(0x497178_u32.wrapping_add(offset))?);
            m.set_u16(r.edx, r.ebx as u16)?;
            let stone = m.u16(0x497176_u32.wrapping_add(offset))?;
            r.edx = (r.edx & 0xffff0000) | u32::from(stone);
            m.set_u16(stone_output, stone)?;
            r.eax = (index & 0xffff0000) | u32::from(m.u16(0x49717a_u32.wrapping_add(offset))?);
            m.set_u16(r.ecx, r.eax as u16)?;
        }
        _ => return Err(EngineError::UnknownRoutine(address)),
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
