//! CD music presentation policy, ported from `NativeCDMusicHost.swift`.
//!
//! Tracks 2 and 3 are imported from the player's raw `.BIN` only (the recipe
//! manifest refuses `music/*.wav` when the source has no raw image). The engine
//! requests playback through frame-event kind 2 id 102; the host plays the
//! first request while idle and never invents a track from a stage, a guard
//! byte or a local random source.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::host::events::MusicRequest;
use crate::platform::audio::AudioHandle;

const SITES: [u32; 2] = [0x0042_d32d, 0x0046_84bd];

pub struct MusicPlayer {
    tracks: [Option<PathBuf>; 2],
    stage_active: bool,
    automatic: bool,
    muted: bool,
    current: u8,
    rejected: BTreeSet<u8>,
    transport_error: Option<i32>,
    pub requests: u64,
    pub suppressed: u64,
    pub starts: u64,
    pub failures: u64,
    pub notice: String,
}

impl MusicPlayer {
    /// Builds a catalog from the imported data folder. A missing track stays a
    /// track-specific failure; the rest of the game is unaffected.
    pub fn from_imported(folder: &Path) -> Self {
        let track = |number: u8| {
            let path = folder.join(format!("music/TRACK{number:02}.wav"));
            path.is_file().then_some(path)
        };
        Self {
            tracks: [track(2), track(3)],
            stage_active: false,
            automatic: true,
            muted: false,
            current: 0,
            rejected: BTreeSet::new(),
            transport_error: None,
            requests: 0,
            suppressed: 0,
            starts: 0,
            failures: 0,
            notice: "Música do CD · aguardando pedido do jogo.".into(),
        }
    }

    pub fn begin_stage(&mut self) {
        self.stage_active = true;
        self.automatic = true;
        self.rejected.clear();
        self.transport_error = None;
    }

    pub fn result(&mut self) {
        self.stage_active = false;
    }

    pub fn leave_stage(&mut self, audio: &AudioHandle) {
        self.stage_active = false;
        audio.stop_music();
        self.current = 0;
    }

    pub fn stop(&mut self, audio: &AudioHandle) {
        audio.stop_music();
        self.current = 0;
        self.notice = "Música do CD parada.".into();
    }

    pub fn mute(&mut self, value: bool) {
        self.muted = value;
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    pub fn track(&self) -> u8 {
        self.current
    }

    pub fn notice(&self) -> &str {
        &self.notice
    }

    /// Applies one original MCICD_Play request.
    pub fn request(&mut self, request: MusicRequest, audio: &mut AudioHandle) {
        if !self.stage_active || !self.automatic {
            return;
        }
        self.requests += 1;
        if !SITES.contains(&request.site) || request.looping {
            self.transport_failed(-1);
            return;
        }
        // 0x42d32d is the stage loop's request: it never restarts the track.
        if request.site == SITES[0] && self.current != 0 {
            self.suppressed += 1;
            return;
        }
        if self.rejected.contains(&request.track) {
            return;
        }
        let index = match request.track {
            2 => 0,
            3 => 1,
            _ => {
                self.rejected.insert(request.track);
                self.failures += 1;
                self.notice = format!("Faixa de CD não suportada: {}.", request.track);
                return;
            }
        };
        let Some(path) = self.tracks[index].clone() else {
            self.rejected.insert(request.track);
            self.failures += 1;
            self.notice = format!(
                "Arquivo de música original da faixa {} ausente ou inacessível.",
                request.track
            );
            return;
        };
        if audio.play_music(&path, false) {
            self.current = request.track;
            self.starts += 1;
            self.notice = format!("Música do CD · faixa {}", request.track);
        } else {
            self.rejected.insert(request.track);
            self.failures += 1;
            self.notice = format!(
                "Não foi possível reproduzir a faixa original {} do CD.",
                request.track
            );
        }
    }

    pub fn transport_failed(&mut self, code: i32) {
        if self.transport_error == Some(code) {
            return;
        }
        self.transport_error = Some(code);
        self.automatic = false;
        self.failures += 1;
        self.notice = format!("Pedidos de música indisponíveis ({code}).");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_catalog_stays_track_specific() {
        let player = MusicPlayer::from_imported(Path::new("/missing/folder"));
        assert!(player.tracks.iter().all(Option::is_none));
        assert_eq!(player.track(), 0);
    }
}
