//! Sound playback for the engine's ordered requests.
//!
//! The samples were decoded from the CD's `.SND` banks at import into mono
//! 22,050 Hz PCM16 WAVs, named `<bank>/<index>-v<variant>.wav`. The `A.HED`,
//! `B.HED` and `P.HED` banks map a `PlaySnd_PostSound` selector + sound id to
//! that file. Mixing is host presentation only and never touches game state.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::host::events::SoundEvent;

#[derive(Clone, Debug)]
pub struct Wav {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}

impl Wav {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }

    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return None;
        }
        let mut offset = 12usize;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits = 0u16;
        let mut format = 0u16;
        while offset + 8 <= bytes.len() {
            let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?) as usize;
            let body = offset + 8;
            if &bytes[offset..offset + 4] == b"fmt " {
                if size < 16 || body + 16 > bytes.len() {
                    return None;
                }
                format = u16::from_le_bytes(bytes[body..body + 2].try_into().ok()?);
                channels = u16::from_le_bytes(bytes[body + 2..body + 4].try_into().ok()?);
                sample_rate =
                    u32::from_le_bytes(bytes[body + 4..body + 8].try_into().ok()?);
                bits = u16::from_le_bytes(bytes[body + 14..body + 16].try_into().ok()?);
            } else if &bytes[offset..offset + 4] == b"data" {
                if format != 1 || bits != 16 || channels == 0 || sample_rate == 0 {
                    return None;
                }
                if body + size > bytes.len() {
                    return None;
                }
                let samples = bytes[body..body + size]
                    .chunks_exact(2)
                    .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                    .collect();
                return Some(Self {
                    channels,
                    sample_rate,
                    samples,
                });
            }
            offset = body + size + (size & 1);
        }
        None
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        Self::parse(&std::fs::read(path).ok()?)
    }
}

/// The three imported HED banks and their sample files.
pub struct SoundCatalog {
    root: PathBuf,
    banks: [Option<Vec<u8>>; 3],
    cache: BTreeMap<PathBuf, Option<Arc<Wav>>>,
}

const SELECTORS: [u8; 3] = [0x41, 0x42, 0x50];
const LETTERS: [&str; 3] = ["A", "B", "P"];

impl SoundCatalog {
    pub fn open(root: &Path) -> Self {
        let banks = std::array::from_fn(|index| {
            std::fs::read(root.join(format!("{}.HED", LETTERS[index])))
                .ok()
                .filter(|bytes| bytes.len() >= 16 && &bytes[..4] == b"ABCD")
        });
        Self {
            root: root.to_path_buf(),
            banks,
            cache: BTreeMap::new(),
        }
    }

    pub fn available(&self) -> bool {
        self.banks.iter().all(Option::is_some)
    }

    fn bank(&self, selector: u32) -> Option<(usize, &[u8])> {
        let index = SELECTORS
            .iter()
            .position(|&value| value as u32 == selector)?;
        self.banks[index].as_deref().map(|body| (index, body))
    }

    /// Maps `(selector, id)` to `<bank>/<index>-v<variant>.wav`.
    pub fn sample(&mut self, selector: u32, id: u32, counter: u32) -> Option<(PathBuf, u32)> {
        let (bank, hed) = self.bank(selector)?;
        if hed.len() < 16 {
            return None;
        }
        let count = u32::from_le_bytes([hed[12], hed[13], hed[14], hed[15]]) as usize;
        for index in 0..count {
            let start = 16 + index * 12;
            if start + 12 > hed.len() {
                break;
            }
            let entry_id = u32::from_le_bytes(hed[start..start + 4].try_into().ok()?);
            if entry_id != id {
                continue;
            }
            let variants =
                u16::from_le_bytes(hed[start + 6..start + 8].try_into().ok()?) as u32;
            let variant = if variants > 1 {
                counter & (variants - 1)
            } else {
                0
            };
            let path = self.root.join(format!(
                "{}/{:04}-v{:02}.wav",
                LETTERS[bank], index, variant
            ));
            return Some((path, variant));
        }
        None
    }

    pub fn load(&mut self, path: &Path) -> Option<Arc<Wav>> {
        if let Some(cached) = self.cache.get(path) {
            return cached.clone();
        }
        let wav = Wav::from_path(path).map(Arc::new);
        self.cache.insert(path.to_path_buf(), wav.clone());
        wav
    }
}

struct Voice {
    wav: Arc<Wav>,
    position: f64,
    volume: f32,
    looping: bool,
}

/// Mixer state shared with the audio device thread.
pub struct Mixer {
    catalog: SoundCatalog,
    voices: BTreeMap<u32, Voice>,
    music: Option<Voice>,
    counters: BTreeMap<(u32, u32), u32>,
    pub posts: u64,
    pub stops: u64,
    pub missing: u64,
}

impl Mixer {
    pub fn new(audio_root: &Path) -> Self {
        Self {
            catalog: SoundCatalog::open(audio_root),
            voices: BTreeMap::new(),
            music: None,
            counters: BTreeMap::new(),
            posts: 0,
            stops: 0,
            missing: 0,
        }
    }

    pub fn available(&self) -> bool {
        self.catalog.available()
    }

    fn gain(hundredths_db: i32) -> f32 {
        if hundredths_db <= -10000 {
            0.0
        } else {
            10f32.powf(hundredths_db.min(0) as f32 / 2000.0)
        }
    }

    /// Applies one ordered request from `host::events::sound_events`.
    pub fn apply(&mut self, event: &SoundEvent) {
        match event.kind {
            0 => {}
            2 => {
                self.stops += 1;
                self.voices.remove(&event.eax);
            }
            1 => {
                self.posts += 1;
                let volume = event.volume as i32;
                if volume == 0 {
                    self.voices.remove(&event.ebx);
                    return;
                }
                let key = (event.eax, event.edx);
                let counter = self.counters.entry(key).or_insert(0);
                let counter_value = *counter;
                *counter = counter.wrapping_add(1);
                let Some((path, _variant)) = self.catalog.sample(event.eax, event.edx, counter_value)
                else {
                    self.missing += 1;
                    return;
                };
                let Some(wav) = self.catalog.load(&path) else {
                    self.missing += 1;
                    return;
                };
                self.voices.insert(
                    event.ebx,
                    Voice {
                        wav,
                        position: 0.0,
                        volume: Self::gain(volume),
                        looping: event.ecx & 1 != 0,
                    },
                );
            }
            _ => {}
        }
    }

    pub fn set_music(&mut self, wav: Arc<Wav>, volume: f32, looping: bool) {
        self.music = Some(Voice {
            wav,
            position: 0.0,
            volume,
            looping,
        });
    }

    pub fn stop_music(&mut self) {
        self.music = None;
    }

    pub fn music_playing(&self) -> bool {
        self.music.is_some()
    }

    /// Renders interleaved stereo float frames.
    pub fn render(&mut self, frames: usize, sample_rate: u32) -> Vec<f32> {
        let mut out = vec![0.0f32; frames * 2];
        for voice in self.voices.values_mut() {
            mix_voice(voice, &mut out, frames, sample_rate);
        }
        if let Some(music) = self.music.as_mut() {
            mix_voice(music, &mut out, frames, sample_rate);
        }
        self.voices.retain(|_, voice| voice.position < voice.wav.frames() as f64);
        if let Some(music) = &self.music {
            if music.position >= music.wav.frames() as f64 {
                self.music = None;
            }
        }
        out
    }
}

fn mix_voice(voice: &mut Voice, out: &mut [f32], frames: usize, sample_rate: u32) {
    let channels = voice.wav.channels.max(1) as usize;
    let total = voice.wav.frames() as f64;
    if total <= 0.0 {
        return;
    }
    let step = voice.wav.sample_rate as f64 / sample_rate.max(1) as f64;
    for frame in 0..frames {
        if voice.position >= total {
            if voice.looping {
                voice.position %= total;
            } else {
                break;
            }
        }
        let index = voice.position as usize * channels;
        if index + channels > voice.wav.samples.len() {
            break;
        }
        let sample = voice.wav.samples[index] as f32 / 32768.0 * voice.volume;
        out[frame * 2] += sample;
        out[frame * 2 + 1] += sample;
        voice.position += step;
    }
}

/// A running audio device. The game thread posts requests; the device thread
/// renders and writes them, so the engine never waits for the sound card.
pub struct AudioHandle {
    mixer: Arc<Mutex<Mixer>>,
    stop: Arc<AtomicBool>,
    #[cfg(windows)]
    thread: Option<std::thread::JoinHandle<()>>,
    #[cfg(windows)]
    music_wav: Option<Arc<Wav>>,
}

impl AudioHandle {
    pub fn start(audio_root: &Path) -> Option<Self> {
        let mixer = Arc::new(Mutex::new(Mixer::new(audio_root)));
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        {
            let thread_mixer = Arc::clone(&mixer);
            let thread_stop = Arc::clone(&stop);
            let thread = std::thread::Builder::new()
                .name("rising-lands-audio".into())
                .spawn(move || {
                    if let Err(error) = device::run(thread_mixer, thread_stop) {
                        eprintln!("audio device unavailable: {error}");
                    }
                })
                .ok()?;
            Some(Self {
                mixer,
                stop,
                thread: Some(thread),
                music_wav: None,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = audio_root;
            Some(Self {
                mixer,
                stop,
            })
        }
    }

    pub fn available(&self) -> bool {
        self.mixer
            .lock()
            .map(|mixer| mixer.available())
            .unwrap_or(false)
    }

    pub fn post(&self, event: &SoundEvent) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.apply(event);
        }
    }

    /// Loads and starts one imported music track (`music/TRACK02.wav`...).
    pub fn play_music(&mut self, path: &Path, looping: bool) -> bool {
        let Some(wav) = Wav::from_path(path) else {
            return false;
        };
        let wav = Arc::new(wav);
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_music(Arc::clone(&wav), 1.0, looping);
        }
        #[cfg(windows)]
        {
            self.music_wav = Some(wav);
        }
        true
    }

    pub fn stop_music(&self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.stop_music();
        }
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        #[cfg(windows)]
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
mod device {
    use super::*;
    use windows::core::Result as WinResult;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioClient, IAudioRenderClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

    pub fn run(mixer: Arc<Mutex<Mixer>>, stop: Arc<AtomicBool>) -> WinResult<()> {
        // SAFETY: the audio thread owns its COM apartment for its whole life.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let format = client.GetMixFormat()?;
            let event = CreateEventW(None, false, false, None)?;
            let duration = 100 * 10_000; // 100 ms in 100-ns units
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                duration,
                0,
                format,
                None,
            )?;
            client.SetEventHandle(event)?;
            let render: IAudioRenderClient = client.GetService()?;
            let buffer_frames = client.GetBufferSize()?;
            let channels = (*format).nChannels as usize;
            let rate = (*format).nSamplesPerSec;
            let block = (*format).nBlockAlign as usize;
            let is_float = (*format).wFormatTag == 3;
            client.Start()?;
            while !stop.load(Ordering::Relaxed) {
                let wait = WaitForSingleObject(event, 200);
                if wait != WAIT_OBJECT_0 {
                    continue;
                }
                let padding = client.GetCurrentPadding()?;
                let available = buffer_frames.saturating_sub(padding);
                if available == 0 {
                    continue;
                }
                let target = render.GetBuffer(available)?;
                if target.is_null() {
                    continue;
                }
                let samples = {
                    let mut guard = mixer.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    guard.render(available as usize, rate)
                };
                let bytes =
                    std::slice::from_raw_parts_mut(target, available as usize * block);
                write_frames(bytes, &samples, channels, is_float);
                render.ReleaseBuffer(available, 0)?;
            }
            let _ = client.Stop();
            let _ = CloseHandle(HANDLE(event.0));
        }
        Ok(())
    }

    fn write_frames(bytes: &mut [u8], samples: &[f32], channels: usize, is_float: bool) {
        if is_float {
            for (index, frame) in samples.chunks_exact(2).enumerate() {
                for channel in 0..channels {
                    let value = frame[channel.min(1)].clamp(-1.0, 1.0);
                    let offset = (index * channels + channel) * 4;
                    if offset + 4 <= bytes.len() {
                        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                    }
                }
            }
        } else {
            for (index, frame) in samples.chunks_exact(2).enumerate() {
                for channel in 0..channels {
                    let value = (frame[channel.min(1)].clamp(-1.0, 1.0) * 32767.0) as i16;
                    let offset = (index * channels + channel) * 2;
                    if offset + 2 <= bytes.len() {
                        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pcm16_wav_and_mixes_one_voice() {
        let pcm = [0i16, 16384, -16384, 0];
        let mut bytes = Vec::new();
        let payload: Vec<u8> = pcm.iter().flat_map(|value| value.to_le_bytes()).collect();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&((payload.len() + 36) as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&22050u32.to_le_bytes());
        bytes.extend_from_slice(&44100u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);
        let wav = Wav::parse(&bytes).unwrap();
        assert_eq!(wav.channels, 1);
        assert_eq!(wav.frames(), 4);

        let directory = std::env::temp_dir().join(format!(
            "rising-windows-audio-{}",
            std::process::id()
        ));
        let mix = Mixer::new(&directory);
        let mut mix = mix;
        mix.voices.insert(
            5,
            Voice {
                wav: Arc::new(wav),
                position: 0.0,
                volume: 1.0,
                looping: false,
            },
        );
        let rendered = mix.render(2, 22050);
        assert!(rendered[0] > 0.0);
    }
}
