//! Original movie playback: RIFF/AVI container, Cinepak video and Microsoft
//! ADPCM audio, ported from `NativeDefeatMovie.swift`.
//!
//! The three original movies are Cinepak 640x480 at 15 frames/s with MS ADPCM
//! audio. Modern Windows codec packs do not include Cinepak, so the host
//! decodes the container itself; presentation only, never game state.

use std::ops::Range;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct MovieError(pub String);

impl std::fmt::Display for MovieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MovieError {}

fn failed(message: impl Into<String>) -> MovieError {
    MovieError(message.into())
}

#[derive(Clone, Debug)]
pub struct AudioFormat {
    pub tag: u16,
    pub channels: u16,
    pub rate: u32,
    pub block_align: u16,
    pub bits: u16,
    pub samples_per_block: u16,
    /// MS ADPCM pairs (coef1, coef2), 256 = 1.0.
    pub coefficients: Vec<i16>,
}

#[derive(Clone, Debug)]
pub struct Movie {
    data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub microseconds_per_frame: u32,
    pub video_codec: String,
    pub audio: Option<AudioFormat>,
    pub video_chunks: Vec<Range<usize>>,
    pub audio_chunks: Vec<Range<usize>>,
}

impl Movie {
    pub fn open(path: &Path) -> Result<Self, MovieError> {
        Self::new(std::fs::read(path).map_err(|error| failed(error.to_string()))?)
    }

    pub fn new(data: Vec<u8>) -> Result<Self, MovieError> {
        let u16_at = |offset: usize| -> usize {
            data[offset] as usize | (data[offset + 1] as usize) << 8
        };
        let u32_at = |offset: usize| -> usize { u16_at(offset) | u16_at(offset + 2) << 16 };
        let tag = |offset: usize| -> String {
            String::from_utf8_lossy(&data[offset..offset + 4]).into_owned()
        };
        if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"AVI " {
            return Err(failed("não é um AVI"));
        }
        let mut width = 0usize;
        let mut height = 0usize;
        let mut period = 0usize;
        let mut codec = String::new();
        let mut audio = None;
        let mut types: Vec<String> = Vec::new();
        let mut video = Vec::new();
        let mut sound = Vec::new();
        let mut video_stream = None::<usize>;
        let mut audio_stream = None::<usize>;

        fn walk(
            data: &[u8],
            start: usize,
            end: usize,
            in_movi: bool,
            state: &mut WalkState,
        ) -> Result<(), MovieError> {
            let u16_at = |offset: usize| -> usize {
                data[offset] as usize | (data[offset + 1] as usize) << 8
            };
            let u32_at = |offset: usize| -> usize { u16_at(offset) | u16_at(offset + 2) << 16 };
            let tag = |offset: usize| -> String {
                String::from_utf8_lossy(&data[offset..offset + 4]).into_owned()
            };
            let mut p = start;
            while p + 8 <= end {
                let id = tag(p);
                let size = u32_at(p + 4);
                let body = p + 8;
                if body + size > end {
                    return Err(failed(format!("bloco {id} truncado")));
                }
                if id == "LIST" || id == "RIFF" {
                    if size < 4 {
                        return Err(failed("lista vazia"));
                    }
                    let kind = tag(body);
                    walk(data, body + 4, body + size, in_movi || kind == "movi", state)?;
                } else if in_movi {
                    let stream = id
                        .get(..2)
                        .and_then(|digits| digits.parse::<usize>().ok())
                        .unwrap_or(usize::MAX);
                    if Some(stream) == state.video_stream
                        && (id.ends_with("dc") || id.ends_with("db"))
                    {
                        state.video.push(body..body + size);
                    } else if Some(stream) == state.audio_stream && id.ends_with("wb") {
                        state.sound.push(body..body + size);
                    }
                } else if id == "avih" && size >= 40 {
                    state.period = u32_at(body);
                    state.width = u32_at(body + 32);
                    state.height = u32_at(body + 36);
                } else if id == "strh" && size >= 8 {
                    state.types.push(tag(body));
                    state.handlers.push(tag(body + 4));
                } else if id == "strf" {
                    if let Some(type_) = state.types.last() {
                        let index = state.types.len() - 1;
                        if type_ == "vids" && state.video_stream.is_none() && size >= 20 {
                            state.video_stream = Some(index);
                            state.codec = tag(body + 16);
                            state.width = u32_at(body + 4);
                            state.height = u32_at(body + 8);
                        } else if type_ == "auds" && state.audio_stream.is_none() && size >= 16 {
                            state.audio_stream = Some(index);
                            let format_tag = u16_at(body) as u16;
                            let channels = u16_at(body + 2) as u16;
                            let align = u16_at(body + 12) as u16;
                            let mut per_block = 0u16;
                            let mut coefficients = Vec::new();
                            if format_tag == 2 {
                                if size < 22 {
                                    return Err(failed("formato ADPCM incompleto"));
                                }
                                per_block = u16_at(body + 18) as u16;
                                let count = u16_at(body + 20) as usize;
                                if size < 22 + 4 * count || count == 0 {
                                    return Err(failed("coeficientes ADPCM ausentes"));
                                }
                                for index in 0..count * 2 {
                                    coefficients.push(u16_at(body + 22 + 2 * index) as u16 as i16);
                                }
                            }
                            state.audio = Some(AudioFormat {
                                tag: format_tag,
                                channels,
                                rate: u32_at(body + 4) as u32,
                                block_align: align,
                                bits: u16_at(body + 14) as u16,
                                samples_per_block: per_block,
                                coefficients,
                            });
                        }
                    }
                }
                p = body + size + (size & 1);
            }
            Ok(())
        }

        struct WalkState {
            width: usize,
            height: usize,
            period: usize,
            codec: String,
            types: Vec<String>,
            handlers: Vec<String>,
            video: Vec<Range<usize>>,
            sound: Vec<Range<usize>>,
            video_stream: Option<usize>,
            audio_stream: Option<usize>,
            audio: Option<AudioFormat>,
        }
        let mut state = WalkState {
            width,
            height,
            period,
            codec,
            types,
            handlers: Vec::new(),
            video,
            sound,
            video_stream,
            audio_stream,
            audio,
        };
        let riff_end = (8 + u32_at(4)).min(data.len());
        walk(&data, 12, riff_end, false, &mut state)?;
        width = state.width;
        height = state.height;
        period = state.period;
        codec = state.codec;
        audio = state.audio;
        video = state.video;
        sound = state.sound;
        video_stream = state.video_stream;
        audio_stream = state.audio_stream;
        if video_stream.is_none() || video.is_empty() {
            return Err(failed("sem trilha de vídeo"));
        }
        if codec.to_ascii_lowercase() != "cvid" {
            return Err(failed(format!("codec de vídeo {codec} não suportado")));
        }
        if width == 0
            || height == 0
            || width % 4 != 0
            || width > 4096
            || height > 4096
            || period == 0
        {
            return Err(failed("dimensões inválidas"));
        }
        if let Some(audio) = &audio {
            if !(audio.channels == 1 || audio.channels == 2) || audio.rate == 0 {
                return Err(failed("áudio inválido"));
            }
            let adpcm = audio.tag == 2 && audio.bits == 4 && audio.block_align >= 7 * audio.channels;
            let pcm = audio.tag == 1 && audio.bits == 16;
            if !adpcm && !pcm {
                return Err(failed(format!(
                    "formato de áudio {} não suportado",
                    audio.tag
                )));
            }
        }
        let _ = audio_stream;
        Ok(Self {
            data,
            width,
            height,
            microseconds_per_frame: period as u32,
            video_codec: codec,
            audio,
            video_chunks: video,
            audio_chunks: sound,
        })
    }

    pub fn frame_count(&self) -> usize {
        self.video_chunks.len()
    }

    pub fn video_duration(&self) -> f64 {
        self.video_chunks.len() as f64 * self.microseconds_per_frame as f64 / 1_000_000.0
    }

    pub fn frame(&self, index: usize) -> Option<&[u8]> {
        self.video_chunks
            .get(index)
            .map(|range| &self.data[range.clone()])
    }

    /// Interleaved signed 16-bit PCM of the whole audio stream.
    pub fn pcm(&self) -> Result<Vec<i16>, MovieError> {
        let Some(audio) = &self.audio else {
            return Ok(Vec::new());
        };
        let mut stream = Vec::new();
        for chunk in &self.audio_chunks {
            stream.extend_from_slice(&self.data[chunk.clone()]);
        }
        if audio.tag == 1 {
            return Ok(stream
                .chunks_exact(2)
                .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                .collect());
        }
        decode_ms_adpcm(&stream, audio)
    }

    pub fn audio_rate(&self) -> u32 {
        self.audio.as_ref().map(|audio| audio.rate).unwrap_or(0)
    }

    pub fn audio_channels(&self) -> u16 {
        self.audio.as_ref().map(|audio| audio.channels).unwrap_or(0)
    }
}

const ADAPTATION: [i32; 16] = [
    230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230,
];

pub fn decode_ms_adpcm(bytes: &[u8], format: &AudioFormat) -> Result<Vec<i16>, MovieError> {
    let channels = format.channels as usize;
    let header = 7 * channels;
    let pairs = format.coefficients.len() / 2;
    let mut output = Vec::with_capacity(bytes.len() * 2);
    let mut block = 0usize;
    while block + header <= bytes.len() {
        let end = (block + format.block_align as usize).min(bytes.len());
        let mut samples = (end - block - header) * 2 / channels + 2;
        if format.samples_per_block > 0 {
            samples = samples.min(format.samples_per_block as usize);
        }
        let mut c1 = [0i32; 2];
        let mut c2 = [0i32; 2];
        let mut delta = [0i32; 2];
        let mut s1 = [0i32; 2];
        let mut s2 = [0i32; 2];
        let s16 = |offset: usize| -> i32 {
            i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32
        };
        for channel in 0..channels {
            let predictor = bytes[block + channel] as usize;
            if predictor >= pairs {
                return Err(failed(format!("preditor ADPCM {predictor} inválido")));
            }
            c1[channel] = format.coefficients[2 * predictor] as i32;
            c2[channel] = format.coefficients[2 * predictor + 1] as i32;
            delta[channel] = s16(block + channels + 2 * channel);
            s1[channel] = s16(block + 3 * channels + 2 * channel);
            s2[channel] = s16(block + 5 * channels + 2 * channel);
        }
        for channel in 0..channels {
            output.push(s2[channel] as i16);
        }
        for channel in 0..channels {
            output.push(s1[channel] as i16);
        }
        let mut p = block + header;
        for _ in 0..((samples - 2) * channels / 2) {
            if p >= end {
                break;
            }
            let byte = bytes[p] as i32;
            p += 1;
            output.push(expand(&mut c1, &mut c2, &mut delta, &mut s1, &mut s2, 0, byte >> 4));
            output.push(expand(
                &mut c1,
                &mut c2,
                &mut delta,
                &mut s1,
                &mut s2,
                channels - 1,
                byte & 15,
            ));
        }
        block += format.block_align as usize;
    }
    Ok(output)
}

fn expand(
    c1: &mut [i32; 2],
    c2: &mut [i32; 2],
    delta: &mut [i32; 2],
    s1: &mut [i32; 2],
    s2: &mut [i32; 2],
    channel: usize,
    nibble: i32,
) -> i16 {
    let mut predicted =
        (s1[channel].wrapping_mul(c1[channel]) + s2[channel].wrapping_mul(c2[channel])) / 256;
    predicted += if nibble >= 8 { nibble - 16 } else { nibble } * delta[channel];
    predicted = predicted.clamp(-32768, 32767);
    s2[channel] = s1[channel];
    s1[channel] = predicted;
    delta[channel] = (16).max(
        (i32::MAX / 768).min((ADAPTATION[nibble as usize] * delta[channel]) >> 8),
    );
    predicted as i16
}

/// Cinepak decoder into RGBX pixels (4 bytes per pixel, X = 255). Codebooks
/// persist per strip across frames; inter frames update in place.
pub struct CinepakDecoder {
    width: usize,
    height: usize,
    pub pixels: Vec<u8>,
    v4: Vec<u8>,
    v1: Vec<u8>,
}

const BOOK: usize = 256 * 16;
const MAX_STRIPS: usize = 32;

impl CinepakDecoder {
    pub fn new(width: usize, height: usize) -> Self {
        let mut pixels = vec![0u8; width * height * 4];
        for index in (3..pixels.len()).step_by(4) {
            pixels[index] = 255;
        }
        let v4 = vec![0u8; MAX_STRIPS * BOOK];
        let v1 = v4.clone();
        Self {
            width,
            height,
            pixels,
            v4,
            v1,
        }
    }

    pub fn decode(&mut self, frame: &[u8]) -> Result<(), MovieError> {
        let be16 = |offset: usize| -> usize { (frame[offset] as usize) << 8 | frame[offset + 1] as usize };
        let be24 = |offset: usize| -> usize {
            (frame[offset] as usize) << 16 | be16(offset + 1)
        };
        if frame.len() < 10 {
            return Err(failed("quadro Cinepak curto"));
        }
        let flags = frame[0];
        let strips = be16(8).min(MAX_STRIPS);
        let mut p = 10usize;
        let mut y0 = 0usize;
        for strip in 0..strips {
            if p + 12 > frame.len() {
                return Err(failed("faixa Cinepak truncada"));
            }
            let mut size = be24(p + 1).wrapping_sub(12);
            let y1 = y0;
            let y2 = y0 + be16(p + 8);
            p += 12;
            size = size.min(frame.len().saturating_sub(p));
            if strip > 0 && flags & 1 == 0 {
                let from = (strip - 1) * BOOK;
                let to = strip * BOOK;
                self.v4.copy_within(from..from + BOOK, to);
                self.v1.copy_within(from..from + BOOK, to);
            }
            self.decode_strip(strip, y1, y2, frame, p, p + size)?;
            p += size;
            y0 = y2;
        }
        Ok(())
    }

    fn decode_strip(
        &mut self,
        strip: usize,
        y1: usize,
        y2: usize,
        frame: &[u8],
        mut p: usize,
        end: usize,
    ) -> Result<(), MovieError> {
        while p + 4 <= end {
            let id = frame[p] as usize;
            let declared = (frame[p + 1] as usize) << 16
                | (frame[p + 2] as usize) << 8
                | frame[p + 3] as usize;
            let size = declared.saturating_sub(4).min(end.saturating_sub(p + 4));
            p += 4;
            match id {
                0x20 | 0x21 | 0x24 | 0x25 => {
                    codebook(&mut self.v4, strip, id, frame, p, p + size);
                }
                0x22 | 0x23 | 0x26 | 0x27 => {
                    codebook(&mut self.v1, strip, id, frame, p, p + size);
                }
                0x30 | 0x31 | 0x32 => {
                    self.vectors(strip, id, y1, y2, frame, p, p + size)?;
                    return Ok(());
                }
                _ => {}
            }
            p += size;
        }
        Ok(())
    }

    fn vectors(
        &mut self,
        strip: usize,
        id: usize,
        y1: usize,
        y2: usize,
        frame: &[u8],
        mut p: usize,
        end: usize,
    ) -> Result<(), MovieError> {
        let stride4 = self.width * 4;
        let cb = strip * BOOK;
        let mut flag: u32 = 0;
        let mut mask: u32 = 0;
        macro_rules! read {
            () => {{
                if p + 4 > end {
                    return Err(failed("vetores Cinepak truncados"));
                }
                flag = (frame[p] as u32) << 24
                    | (frame[p + 1] as u32) << 16
                    | (frame[p + 2] as u32) << 8
                    | frame[p + 3] as u32;
                p += 4;
                mask = 0x8000_0000;
            }};
        }
        let mut y = y1;
        while y < y2 {
            let mut x = 0usize;
            while x < self.width {
                let mut coded = true;
                if id & 1 != 0 {
                    mask >>= 1;
                    if mask == 0 {
                        read!();
                    }
                    coded = flag & mask != 0;
                }
                if coded {
                    let mut single = id & 2 != 0;
                    if !single {
                        mask >>= 1;
                        if mask == 0 {
                            read!();
                        }
                        single = flag & mask == 0;
                    }
                    if single {
                        if p >= end {
                            return Err(failed("vetor V1 truncado"));
                        }
                        let entry = cb + frame[p] as usize * 16;
                        p += 1;
                        for k in 0..4 {
                            let bx = x + (k & 1) * 2;
                            let by = y + (k >> 1) * 2;
                            for d in 0..4 {
                                put(
                                    &mut self.pixels,
                                    self.width,
                                    self.height,
                                    &self.v1,
                                    entry + 4 * k,
                                    bx + (d & 1),
                                    by + (d >> 1),
                                );
                            }
                        }
                    } else {
                        if p + 4 > end {
                            return Err(failed("vetor V4 truncado"));
                        }
                        for k in 0..4 {
                            let entry = cb + frame[p + k] as usize * 16;
                            let bx = x + (k & 1) * 2;
                            let by = y + (k >> 1) * 2;
                            for d in 0..4 {
                                put(
                                    &mut self.pixels,
                                    self.width,
                                    self.height,
                                    &self.v4,
                                    entry + 4 * d,
                                    bx + (d & 1),
                                    by + (d >> 1),
                                );
                            }
                        }
                        p += 4;
                    }
                }
                x += 4;
            }
            y += 4;
        }
        let _ = stride4;
        Ok(())
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

fn codebook(book: &mut [u8], strip: usize, id: usize, frame: &[u8], start: usize, end: usize) {
    let n = if id & 4 != 0 { 4 } else { 6 };
    let partial = id & 1 != 0;
    let mut p = start;
    let mut flag: u32 = 0;
    let mut mask: u32 = 0;
    for entry in 0..256 {
        if partial {
            mask >>= 1;
            if mask == 0 {
                if p + 4 > end {
                    return;
                }
                flag = (frame[p] as u32) << 24
                    | (frame[p + 1] as u32) << 16
                    | (frame[p + 2] as u32) << 8
                    | frame[p + 3] as u32;
                p += 4;
                mask = 0x8000_0000;
            }
            if flag & mask == 0 {
                continue;
            }
        }
        if p + n > end {
            return;
        }
        let base = strip * BOOK + entry * 16;
        let u = if n == 6 {
            frame[p + 4] as i8 as i32
        } else {
            0
        };
        let v = if n == 6 {
            frame[p + 5] as i8 as i32
        } else {
            0
        };
        for k in 0..4 {
            let y = frame[p + k] as i32;
            book[base + 4 * k] = (y + v * 2).clamp(0, 255) as u8;
            book[base + 4 * k + 1] = (y - u / 2 - v).clamp(0, 255) as u8;
            book[base + 4 * k + 2] = (y + u * 2).clamp(0, 255) as u8;
            book[base + 4 * k + 3] = 255;
        }
        p += n;
    }
}

fn put(pixels: &mut [u8], width: usize, height: usize, book: &[u8], from: usize, x: usize, y: usize) {
    if y >= height || x >= width || from + 3 > book.len() {
        return;
    }
    let offset = y * width * 4 + x * 4;
    if offset + 3 < pixels.len() {
        pixels[offset] = book[from];
        pixels[offset + 1] = book[from + 1];
        pixels[offset + 2] = book[from + 2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_adpcm_rejects_an_invalid_predictor() {
        let format = AudioFormat {
            tag: 2,
            channels: 1,
            rate: 22050,
            block_align: 8,
            bits: 4,
            samples_per_block: 2,
            coefficients: vec![1, 2],
        };
        assert!(decode_ms_adpcm(&[9, 0, 0, 0, 0, 0, 0, 0], &format).is_err());
    }

    #[test]
    fn cinepak_rejects_a_short_frame() {
        let mut decoder = CinepakDecoder::new(8, 8);
        assert!(decoder.decode(&[0; 4]).is_err());
    }
}
