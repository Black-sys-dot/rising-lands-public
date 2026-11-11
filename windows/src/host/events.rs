//! Transport of engine frame events to the platform layer.
//!
//! Sound requests are the engine's ordered kind-10 `PlaySnd_PostSound` /
//! `PlaySnd_StopSound` copies plus the WinMain loop's `PlaySnd_Manage` ticks;
//! CD music requests are frame-event kind 2 id 102 (MCICD_Play). Nothing here
//! reads or writes game state.

use crate::abi::{RlFrameEvent, RlPlayerInputEvent, RlEventFlcStateV1};

pub const SOUND_MANAGE_SITE: u32 = 0x0046_0220;
pub const EVENT_MCICD_PLAY: u32 = 102;

/// One request for the audio mapper. Layout matches `MacOSSoundEvent`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SoundEvent {
    pub kind: u32,
    pub frame: u32,
    pub phase: u32,
    pub site: u32,
    pub eax: u32,
    pub edx: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub volume: u32,
    pub has_volume: u32,
}

/// One CD-audio request from the original MCICD path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicRequest {
    pub site: u32,
    pub track: u8,
    pub looping: bool,
}

/// Copies the engine's ordered sound requests for one input phase.
pub fn sound_events(events: &[RlPlayerInputEvent], frame: u32, phase: u32) -> Vec<SoundEvent> {
    let mut out = Vec::new();
    if phase == 2 {
        out.push(SoundEvent {
            kind: 0,
            frame,
            phase: 2,
            site: SOUND_MANAGE_SITE,
            ..Default::default()
        });
    }
    let mut requests = 0;
    for event in events.iter().filter(|event| event.kind == 10) {
        out.push(SoundEvent {
            kind: if event.values[0] != 0 { 1 } else { 2 },
            frame: event.frame,
            phase: event.phase,
            site: event.site,
            eax: event.values[1],
            edx: event.values[2],
            ebx: event.values[3],
            ecx: event.values[4],
            volume: event.values[6],
            has_volume: event.values[5],
        });
        requests += 1;
    }
    if phase == 2 && requests != 0 {
        out.push(SoundEvent {
            kind: 0,
            frame,
            phase: 2,
            site: SOUND_MANAGE_SITE,
            ..Default::default()
        });
    }
    out
}

/// MCICD_Play requests: `registers[0]` is the track, `registers[1] & 0xff`
/// the loop flag. The engine never invents a selection.
pub fn music_requests(events: &[RlFrameEvent]) -> Vec<MusicRequest> {
    events
        .iter()
        .filter(|event| event.kind == 2 && event.id == EVENT_MCICD_PLAY)
        .map(|event| MusicRequest {
            site: event.site,
            track: (event.registers[0] & 0xff) as u8,
            looping: event.registers[1] & 0xff != 0,
        })
        .collect()
}

/// In-game FLC event state, as published by `rl_evento_flc_estado_v1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventFlcState {
    pub running: bool,
    pub modal: bool,
    pub index: u32,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
}

impl From<RlEventFlcStateV1> for EventFlcState {
    fn from(state: RlEventFlcStateV1) -> Self {
        Self {
            running: state.running != 0,
            modal: state.modal != 0,
            index: state.index,
            frames: state.frames,
            width: state.width,
            height: state.height,
            speed: state.speed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player_event() -> RlPlayerInputEvent {
        let mut event = RlPlayerInputEvent::default();
        event.kind = 10;
        event.frame = 7;
        event.phase = 2;
        event.site = 0x1234;
        event.values = [1, 0x41, 12, 0x9000, 1, 1, 0, -100, 0, 0, 0, 0]
            .map(|value| value as u32);
        event
    }

    #[test]
    fn post_sound_keeps_register_order() {
        let events = sound_events(&[player_event()], 7, 2);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].kind, 0);
        assert_eq!(events[1].kind, 1);
        assert_eq!(events[1].eax, 0x41);
        assert_eq!(events[1].edx, 12);
        assert_eq!(events[1].ebx, 0x9000);
        assert_eq!(events[1].ecx, 1);
        assert_eq!(events[1].volume, (-100i32) as u32);
        assert_eq!(events[1].has_volume, 1);
        assert_eq!(events[2].kind, 0);
    }

    #[test]
    fn music_requests_select_track_and_loop() {
        let mut event = RlFrameEvent::default();
        event.kind = 2;
        event.id = EVENT_MCICD_PLAY;
        event.site = 0x42d32d;
        event.registers = [2, 0, 0, 0];
        let requests = music_requests(&[event]);
        assert_eq!(
            requests,
            vec![MusicRequest {
                site: 0x42d32d,
                track: 2,
                looping: false
            }]
        );
    }
}
