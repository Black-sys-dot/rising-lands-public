//! The player edition derives every presentation input from the CD alone. This
//! checks each derived value against the SHA-256 of the input the private build
//! used to prepare (palette, shadow and lighting tables, shell pictures, the
//! bank base of the terrain cache). Runs when RL_CD_ROOT names the original CD
//! (as the gate and validate.sh do); without it there is nothing to compare.
use rl_core::{mission_boot, mission_files::Files, render_assets, shell_picture};
use std::{fs, path::PathBuf};

fn hex(b: &[u8]) -> String {
    render_assets::sha256(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

#[test]
fn cd_derived_presentation_inputs_match_the_prepared_ones() {
    let Some(cd) = std::env::var_os("RL_CD_ROOT").map(PathBuf::from) else {
        eprintln!("RL_CD_ROOT unset: CD-derived input check skipped");
        return;
    };
    let files = Files::read(&cd).unwrap();
    let memory = mission_boot::startup(
        &files.assets(),
        mission_boot::StartupInput {
            allocation_fill: 0xa7,
            ai_clock_ms: 0x123456,
            thread_id: 1,
        },
    )
    .unwrap();
    let table = |slot: u32, n: usize| memory.read(memory.u32(slot).unwrap(), n).unwrap().to_vec();
    let palette = table(0x4e24b0, 768);
    let shadow = table(0x4e24b4, 256);
    let light = table(0x4e24b8, 256);
    // The prepared palette.bin, asset7.bin (CalculOmbrage capture) and asset8.bin
    // (CalculEclairage PLATGAME 60% capture).
    assert_eq!(
        hex(&palette),
        "3fa7d79f6e8fd177d20be32903eac7fdccf2942abaf4f2ab5b52216475da70be"
    );
    assert_eq!(
        hex(&shadow),
        "52f6d514b6bed2ff3e62c273abf6ccd79082e5205d657fc7d1787f347ee3b5b6"
    );
    assert_eq!(
        hex(&light),
        "e3ae0b131cd8dd65192fb53b83fcac0b0ce194cc87b5a894e301996d00cc6938"
    );

    let graphics = cd.join("INSTALHD/GRAPHICS");
    let mut blobs: Vec<Vec<u8>> = [
        "BLOCS.CMP",
        "TROOPS.CMP",
        "VEHICULE.CMP",
        "CREATURE.CMP",
        "BUILDING.CMP",
        "OTHERS.CMP",
    ]
    .iter()
    .map(|n| fs::read(graphics.join(n)).unwrap())
    .collect();
    blobs.push(fs::read(cd.join("INSTALHD/PLATGAME.PIE")).unwrap());
    blobs.push(shadow);
    blobs.push(light);
    let refs: Vec<&[u8]> = blobs.iter().map(Vec::as_slice).collect();
    let assets = render_assets::Assets::load(&refs).unwrap();
    assert_eq!(
        assets.palette.to_vec(),
        palette,
        "renderer and world palettes agree"
    );
    let cache = assets.initial_cache().unwrap();
    assert!(cache.keys.iter().all(|&k| k == 0) && cache.cursor == 0);
    // base_payloads_sha256 of the M2.2 initial-cache artifact: BLOCS.CMP IDs 1090..1345.
    assert_eq!(
        hex(&cache.raw),
        "aef6ff4ed09dde359ef3dd113638167b63b6fbe600128485b2e560b87c5e0e55"
    );

    for (name, sha) in [
        (
            "TITRE.PIE",
            "8f02db403441a9651db634c5a7d2038834d21990cbfef60ce5c80737e3dae5d6",
        ),
        (
            "PAGEMIS.PIE",
            "66570ec9661c4253ba438446c8ef3e0ccf1508a92fcbcbb6f2988a8677f79bee",
        ),
        (
            "PAGEMIS2.PIE",
            "4a0afb37af133e2e914b19b618b9e326b4b99b0f3818fec720c111e9ed7b04da",
        ),
        (
            "CREDITS.PIE",
            "3805f43540797b5d2503ef317c010cfb1978ecd9d4a6767c538577f85bc5301c",
        ),
        (
            "ECRANFIN.LZS",
            "15330212a1b596eba3995b039886626e3836d00f700250a782965d74f3c42383",
        ),
    ] {
        let file = fs::read(cd.join("INSTALHD").join(name)).unwrap();
        let pixels = shell_picture::decode_shell_file(&file).unwrap();
        assert_eq!(hex(&pixels), sha, "{name}");
    }
}
