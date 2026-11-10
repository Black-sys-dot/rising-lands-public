//! Intact InputGame 0x4298cc..0x4299ad: undiscovered-cell routing.
//! Discovery uses the local player; SelectionExiste uses the invoking player.
//! No RNG, key query, text lookup or order callback is hidden in this provider.
use crate::{frame::stop, frame_core, MemoriaJogo, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Terrain,
    Finish,
}

/// SelectionExiste 0x47253d. SETNE changes AL only, retaining count's AH.
pub fn selection_exists(m: &MemoriaJogo, eax: u32) -> Result<u32> {
    let count = m.u16(0x508728 + ((eax & 255) << 9))?;
    Ok(u32::from(count & 0xff00) | u32::from(count != 0))
}

/// Returns the original continuation, with exact ordered cursor stores.
/// Unsupported order/drag continuations and undeclared memory fail atomically.
/// The caller still owns the common tooltip publication and rain continuation.
pub fn route(m: &mut MemoriaJogo, cell: u32, player: u8) -> Result<Route> {
    let local = u32::from(m.u8(0x508041)?);
    if frame_core::call(m, "CaseEstDecouverte", &[cell & 65535, local])? as u8 != 0 {
        return Ok(Route::Terrain);
    }
    let page = m.u16(0x4e05ae)?;
    if page == 5 || m.u8(0x496208)? == 0 || page == 13 {
        return Ok(Route::Terrain);
    }
    let cursor = if selection_exists(m, u32::from(player))? as u8 == 0 {
        None
    } else if page == 0 {
        if m.u8(0x5099ae)? != 0 && m.u8(0x5099af)? != 0 {
            return stop("InputGame undiscovered order EnvoieOrdre", 0x45ecd4);
        }
        Some((0x1df5, m.u32(0x508f80)? & 7))
    } else {
        Some((0x1e0e, 0))
    };
    // Native joins at0x42b854. The existing drag tail is outside this port.
    if m.u8(0x4e19b0)? != 0 && m.u8(0x5099ae)? != 0 {
        return stop("InputGame undiscovered drag continuation", 0x42b86e);
    }
    if let Some((icon, phase)) = cursor {
        // Preflight both destinations before publishing either store.
        m.u16(0x49a010)?;
        m.u32(0x49a014)?;
        m.set_u16(0x49a010, icon)?;
        m.set_u32(0x49a014, phase)?;
    }
    Ok(Route::Finish)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> MemoriaJogo {
        let mut m = MemoriaJogo::from_blocks([
            (0x490000, vec![0; 0x80000]),
            (0x6000000, vec![0x10; 65536]),
        ])
        .unwrap();
        m.set_u32(0x507d78, 0x6000000).unwrap();
        m.set_u8(0x496208, 1).unwrap();
        m.set_u16(0x508728, 1).unwrap();
        m
    }
    #[test]
    fn missing_second_store_destination_preserves_first_destination() {
        let mut m = MemoriaJogo::from_blocks([
            (0x508041, vec![0]),
            (0x507d78, 0x6000000u32.to_le_bytes().to_vec()),
            (0x6000000, vec![0x10; 65536]),
            (0x4e05ae, vec![0, 0]),
            (0x496208, vec![1]),
            (0x508728, vec![1, 0]),
            (0x5099ae, vec![0, 0]),
            (0x508f80, vec![7, 0, 0, 0]),
            (0x4e19b0, vec![0]),
            (0x49a010, vec![0xd8, 0x1d]),
        ])
        .unwrap();
        let before = m.clone();
        assert_eq!(
            route(&mut m, 123, 0),
            Err(crate::EngineError::UndeclaredAccess {
                address: 0x49a014,
                size: 4,
            })
        );
        assert_eq!(m, before);
    }
    #[test]
    fn unsupported_native_order_and_drag_preserve_all_bytes() {
        let mut m = context();
        m.set_u8(0x5099ae, 1).unwrap();
        m.set_u8(0x5099af, 1).unwrap();
        let before = m.clone();
        assert!(matches!(
            route(&mut m, 123, 0),
            Err(crate::EngineError::UnportedCallee {
                address: 0x45ecd4,
                ..
            })
        ));
        assert_eq!(m, before);
        m.set_u8(0x5099af, 0).unwrap();
        m.set_u8(0x4e19b0, 1).unwrap();
        let before = m.clone();
        assert!(matches!(
            route(&mut m, 123, 0),
            Err(crate::EngineError::UnportedCallee {
                address: 0x42b86e,
                ..
            })
        ));
        assert_eq!(m, before);
    }
}
