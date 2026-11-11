//! Platform layer. Windowing and menus are Win32-only; the decoders and the
//! import recipes are portable and testable without a device or the CD.

pub mod audio;
pub mod avi;
pub mod cd_import;
pub mod cd_music;
pub mod flc;

#[cfg(windows)]
pub mod menus;
#[cfg(windows)]
pub mod window;

/// Native 800x600 surface geometry shared by the presenters.
pub const SURFACE_WIDTH: usize = 800;
pub const SURFACE_HEIGHT: usize = 600;
pub const SURFACE_PIXELS: usize = SURFACE_WIDTH * SURFACE_HEIGHT;

/// Expands an indexed 800x600 surface through a 768-byte palette into BGRA.
pub fn expand_bgra(indexed: &[u8], palette: &[u8], out: &mut [u8]) {
    assert!(indexed.len() >= SURFACE_PIXELS);
    assert!(palette.len() >= 768);
    assert!(out.len() >= SURFACE_PIXELS * 4);
    for (index, &value) in indexed[..SURFACE_PIXELS].iter().enumerate() {
        let entry = value as usize * 3;
        let destination = index * 4;
        out[destination] = palette[entry + 2];
        out[destination + 1] = palette[entry + 1];
        out[destination + 2] = palette[entry];
        out[destination + 3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_expansion_is_bgra_with_opaque_alpha() {
        let mut indexed = vec![0u8; SURFACE_PIXELS];
        indexed[0] = 1;
        let mut palette = vec![0u8; 768];
        palette[3] = 10;
        palette[4] = 20;
        palette[5] = 30;
        let mut out = vec![0u8; SURFACE_PIXELS * 4];
        expand_bgra(&indexed, &palette, &mut out);
        assert_eq!(&out[..4], &[30, 20, 10, 255]);
        assert_eq!(&out[4..8], &[0, 0, 0, 255]);
    }
}
