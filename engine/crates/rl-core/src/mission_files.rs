//! Filesystem adapter for mission tools. No captured world is read by stage entry.
use crate::{
    mission_boot::{StartupAssets, StartupInput},
    mission_entry::{self, Stage},
    render::{Cache, Renderer},
    render_assets::Assets,
    FrameInput, FrameRegisters, MemoriaJogo,
};
use std::{fs, path::Path, sync::Arc};
pub type HostResult<T> = Result<T, Box<dyn std::error::Error>>;
pub struct Files {
    pub blobs: Vec<Vec<u8>>,
}
impl Files {
    pub fn read(cd: &Path) -> HostResult<Self> {
        Ok(Self {
            blobs: [
                "RISING.EXE",
                "RISING.INI",
                "RISING.RUL",
                "RISING.SBL",
                "TABLEAU.COL",
                "COULEUR.MAP",
                "PLATGAME.PIE",
                "MONSTRE.INI",
                "TEXTES.TXT",
            ]
            .iter()
            .map(|name| fs::read(cd.join("INSTALHD").join(name)))
            .collect::<Result<_, _>>()?,
        })
    }
    pub fn assets(&self) -> StartupAssets<'_> {
        StartupAssets {
            executable: &self.blobs[0],
            rising_ini: &self.blobs[1],
            rules: &self.blobs[2],
            symbols: &self.blobs[3],
            obstacles: &self.blobs[4],
            terrain_colors: &self.blobs[5],
            palette_pie: &self.blobs[6],
        }
    }
    pub fn stage(&self, cd: &Path, name: &str, input: StartupInput) -> HostResult<MemoriaJogo> {
        let stage = Stage::parse(name)?;
        Ok(mission_entry::load_stage(
            &self.assets(),
            input,
            stage,
            &fs::read(cd.join("WORLD").join(name.to_ascii_uppercase()))?,
            &self.blobs[7],
            &self.blobs[8],
        )?)
    }
}
pub fn frame_input(path: &Path) -> HostResult<FrameInput> {
    let b = fs::read(path)?;
    if b.len() != 72 {
        return Err("expected 72-byte host FrameInput".into());
    }
    let v: Vec<_> = b
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    Ok(FrameInput {
        cursor_x: v[0] as i32,
        cursor_y: v[1] as i32,
        offscreen: v[2],
        registers: FrameRegisters {
            eax: v[3],
            ebx: v[4],
            ecx: v[5],
            edx: v[6],
            esi: v[7],
            edi: v[8],
            ebp: v[9],
            esp: v[10],
        },
        saved: v[11..14].try_into().unwrap(),
        msg: v[14..18].try_into().unwrap(),
    })
}
/// Only immutable render assets and the presentation surface/cache are imported.
/// input1/input2 are explicit host call contexts, shared with the S0 control run.
pub struct Presentation {
    pub input: FrameInput,
    pub carried: FrameRegisters,
    pub assets: Arc<Assets>,
    pub surface: Vec<u8>,
    pub cache: Cache,
}
impl Presentation {
    pub fn read(path: &Path) -> HostResult<Self> {
        let blobs: Vec<_> = (0..9)
            .map(|i| fs::read(path.join(format!("asset{i}.bin"))))
            .collect::<Result<_, _>>()?;
        let refs: Vec<_> = blobs.iter().map(Vec::as_slice).collect();
        let result = Self {
            input: frame_input(&path.join("input1.bin"))?,
            carried: frame_input(&path.join("input2.bin"))?.registers,
            assets: Arc::new(Assets::load(&refs).map_err(|e| format!("assets: {e:?}"))?),
            surface: fs::read(path.join("surface.bin"))?,
            cache: Cache {
                keys: fs::read(path.join("keys.bin"))?,
                raw: fs::read(path.join("cache.bin"))?,
                cursor: u32::from_le_bytes(
                    fs::read(path.join("cursor.bin"))?.as_slice().try_into()?,
                )
                .try_into()?,
            },
        };
        result.renderer()?;
        Ok(result)
    }
    pub fn renderer(&self) -> HostResult<Renderer> {
        Renderer::new(Arc::clone(&self.assets), &self.surface, self.cache.clone())
            .map_err(|e| format!("renderer: {e:?}").into())
    }
}
