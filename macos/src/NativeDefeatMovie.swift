import AppKit
import AVFoundation

// Original movies (EVENTS\OVER_FR.AVI, EXTROFR.AVI, INTROFR.AVI): Cinepak 640x480
// video at 15 frames/s with Microsoft ADPCM 22,050 Hz audio. Modern macOS has no
// decoder for either codec, so the host decodes them here. Decoding is host
// presentation only: the engine owns the request and the return to MenuInit2.

/// RIFF/AVI container: stream formats and the byte ranges of every chunk of the
/// first video and first audio stream, in file order.
struct NativeAVIMovie {
    struct Failure: Error, CustomStringConvertible { let description: String }
    struct AudioFormat: Equatable {
        let tag: Int, channels: Int, rate: Int, blockAlign: Int, bits: Int, samplesPerBlock: Int
        let coefficients: [Int32]   // MS ADPCM pairs (coef1, coef2), 256 = 1.0
    }
    let data: Data
    let width: Int, height: Int, microsecondsPerFrame: Int, videoCodec: String
    let audio: AudioFormat?
    let videoChunks: [Range<Int>]
    let audioChunks: [Range<Int>]

    var frameCount: Int { videoChunks.count }
    var videoDuration: Double { Double(videoChunks.count) * Double(microsecondsPerFrame) / 1_000_000 }

    init(data: Data) throws {
        self.data = data
        func fail(_ text: String) -> Failure { Failure(description: text) }
        let bytes = [UInt8](data)
        func u16(_ at: Int) -> Int { Int(bytes[at]) | Int(bytes[at + 1]) << 8 }
        func u32(_ at: Int) -> Int { u16(at) | u16(at + 2) << 16 }
        func tag(_ at: Int) -> String { String(decoding: bytes[at..<at + 4], as: UTF8.self) }
        guard bytes.count >= 12, tag(0) == "RIFF", tag(8) == "AVI " else { throw fail("não é um AVI") }
        var width = 0, height = 0, period = 0, codec = "", audio: AudioFormat?
        var types = [String](), handlers = [String](), video = [Range<Int>](), sound = [Range<Int>]()
        var videoStream = -1, audioStream = -1
        func walk(_ start: Int, _ end: Int, inMovi: Bool) throws {
            var p = start
            while p + 8 <= end {
                let id = tag(p), size = u32(p + 4), body = p + 8
                guard body + size <= end else { throw fail("bloco \(id) truncado") }
                if id == "LIST" || id == "RIFF" {
                    guard size >= 4 else { throw fail("lista vazia") }
                    let kind = tag(body)
                    try walk(body + 4, body + size, inMovi: inMovi || kind == "movi")
                } else if inMovi {
                    let stream = Int(id.prefix(2)) ?? -1
                    if stream == videoStream, id.hasSuffix("dc") || id.hasSuffix("db") { video.append(body..<body + size) }
                    else if stream == audioStream, id.hasSuffix("wb") { sound.append(body..<body + size) }
                } else if id == "avih", size >= 40 {
                    period = u32(body); width = u32(body + 32); height = u32(body + 36)
                } else if id == "strh", size >= 8 {
                    types.append(tag(body)); handlers.append(tag(body + 4))
                } else if id == "strf", let type = types.last {
                    let index = types.count - 1
                    if type == "vids", videoStream < 0, size >= 20 {
                        videoStream = index; codec = tag(body + 16)
                        width = u32(body + 4); height = u32(body + 8)
                    } else if type == "auds", audioStream < 0, size >= 16 {
                        audioStream = index
                        let formatTag = u16(body), channels = u16(body + 2), align = u16(body + 12)
                        var perBlock = 0, coefficients = [Int32]()
                        if formatTag == 2 {
                            guard size >= 22 else { throw fail("formato ADPCM incompleto") }
                            perBlock = u16(body + 18)
                            let count = u16(body + 20)
                            guard size >= 22 + 4 * count, count >= 1 else { throw fail("coeficientes ADPCM ausentes") }
                            for i in 0..<count * 2 { coefficients.append(Int32(Int16(truncatingIfNeeded: u16(body + 22 + 2 * i)))) }
                        }
                        audio = AudioFormat(tag: formatTag, channels: channels, rate: u32(body + 4), blockAlign: align,
                                            bits: u16(body + 14), samplesPerBlock: perBlock, coefficients: coefficients)
                    }
                }
                p = body + size + (size & 1)
            }
        }
        try walk(12, min(bytes.count, 8 + u32(4)), inMovi: false)
        guard videoStream >= 0, !video.isEmpty else { throw fail("sem trilha de vídeo") }
        guard codec.lowercased() == "cvid" else { throw fail("codec de vídeo \(codec) não suportado") }
        guard width > 0, height > 0, width % 4 == 0, width <= 4096, height <= 4096, period > 0 else { throw fail("dimensões inválidas") }
        if let audio {
            guard audio.channels == 1 || audio.channels == 2, audio.rate > 0 else { throw fail("áudio inválido") }
            guard (audio.tag == 2 && audio.bits == 4 && audio.blockAlign >= 7 * audio.channels) || (audio.tag == 1 && audio.bits == 16)
            else { throw fail("formato de áudio \(audio.tag) não suportado") }
        }
        self.width = width; self.height = height; self.microsecondsPerFrame = period; self.videoCodec = codec
        self.audio = audio; self.videoChunks = video; self.audioChunks = audioStream >= 0 ? sound : []
    }

    /// Interleaved signed 16-bit PCM of the whole audio stream.
    func pcm() throws -> [Int16] {
        guard let audio else { return [] }
        var stream = Data(); for chunk in audioChunks { stream.append(data[chunk]) }
        let bytes = [UInt8](stream)
        if audio.tag == 1 {
            return stride(from: 0, to: bytes.count - 1, by: 2).map { Int16(bitPattern: UInt16(bytes[$0]) | UInt16(bytes[$0 + 1]) << 8) }
        }
        return try NativeMSADPCM.decode(bytes, format: audio)
    }

    /// The same PCM as a canonical in-memory WAV for AVAudioPlayer.
    func wav() throws -> Data? {
        guard let audio else { return nil }
        let samples = try pcm()
        var out = Data(capacity: 44 + samples.count * 2)
        func le(_ value: Int, _ size: Int) { for i in 0..<size { out.append(UInt8(truncatingIfNeeded: value >> (8 * i))) } }
        out.append(contentsOf: Array("RIFF".utf8)); le(36 + samples.count * 2, 4); out.append(contentsOf: Array("WAVEfmt ".utf8))
        le(16, 4); le(1, 2); le(audio.channels, 2); le(audio.rate, 4); le(audio.rate * audio.channels * 2, 4); le(audio.channels * 2, 2); le(16, 2)
        out.append(contentsOf: Array("data".utf8)); le(samples.count * 2, 4)
        samples.withUnsafeBytes { out.append(contentsOf: $0) }
        return out
    }
}

/// Microsoft ADPCM (WAVE_FORMAT_ADPCM, tag 2), 4 bits per sample, mono or stereo.
enum NativeMSADPCM {
    static let adaptation: [Int32] = [230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230]

    static func decode(_ bytes: [UInt8], format: NativeAVIMovie.AudioFormat) throws -> [Int16] {
        let channels = format.channels, header = 7 * channels, pairs = format.coefficients.count / 2
        var output = [Int16](); output.reserveCapacity(bytes.count * 2)
        var block = 0
        while block + header <= bytes.count {
            let end = min(block + format.blockAlign, bytes.count)
            // A short final block holds fewer samples, as its byte count says.
            var samples = (end - block - header) * 2 / channels + 2
            if format.samplesPerBlock > 0 { samples = min(samples, format.samplesPerBlock) }
            var c1 = [Int32](repeating: 0, count: 2), c2 = c1, delta = c1, s1 = c1, s2 = c1
            func s16(_ at: Int) -> Int32 { Int32(Int16(bitPattern: UInt16(bytes[at]) | UInt16(bytes[at + 1]) << 8)) }
            for ch in 0..<channels {
                let predictor = Int(bytes[block + ch])
                guard predictor < pairs else { throw NativeAVIMovie.Failure(description: "preditor ADPCM \(predictor) inválido") }
                c1[ch] = format.coefficients[2 * predictor]; c2[ch] = format.coefficients[2 * predictor + 1]
                delta[ch] = s16(block + channels + 2 * ch)
                s1[ch] = s16(block + 3 * channels + 2 * ch)
                s2[ch] = s16(block + 5 * channels + 2 * ch)
            }
            for ch in 0..<channels { output.append(Int16(truncatingIfNeeded: s2[ch])) }
            for ch in 0..<channels { output.append(Int16(truncatingIfNeeded: s1[ch])) }
            func expand(_ ch: Int, _ nibble: Int32) -> Int16 {
                var predicted = (s1[ch] &* c1[ch] &+ s2[ch] &* c2[ch]) / 256
                predicted &+= (nibble >= 8 ? nibble - 16 : nibble) &* delta[ch]
                predicted = max(-32768, min(32767, predicted))
                s2[ch] = s1[ch]; s1[ch] = predicted
                delta[ch] = max(16, min(Int32.max / 768, (adaptation[Int(nibble)] &* delta[ch]) >> 8))
                return Int16(predicted)
            }
            var p = block + header
            for _ in 0..<((samples - 2) * channels / 2) {
                guard p < end else { break }
                let byte = Int32(bytes[p]); p += 1
                output.append(expand(0, byte >> 4))
                output.append(expand(channels - 1, byte & 15))
            }
            block += format.blockAlign
        }
        return output
    }
}

/// Cinepak ("cvid") frame decoder into RGBX pixels (4 bytes per pixel, X = 255).
/// Codebooks persist per strip across frames; inter frames update in place.
final class NativeCinepakDecoder {
    struct Failure: Error, CustomStringConvertible { let description: String }
    static let maxStrips = 32
    let width: Int, height: Int
    private(set) var pixels: [UInt8]
    // Each codebook entry is four RGBX pixels: top-left, top-right, bottom-left, bottom-right.
    private var v4: [UInt8], v1: [UInt8]
    private static let book = 256 * 16

    init(width: Int, height: Int) {
        self.width = width; self.height = height
        pixels = [UInt8](repeating: 0, count: width * height * 4)
        for i in stride(from: 3, to: pixels.count, by: 4) { pixels[i] = 255 }
        v4 = [UInt8](repeating: 0, count: Self.maxStrips * Self.book); v1 = v4
    }

    func decode(_ frame: Data) throws {
        try frame.withUnsafeBytes { raw in try decode(raw.bindMemory(to: UInt8.self)) }
    }

    func decode(_ f: UnsafeBufferPointer<UInt8>) throws {
        func be16(_ p: Int) -> Int { Int(f[p]) << 8 | Int(f[p + 1]) }
        func be24(_ p: Int) -> Int { Int(f[p]) << 16 | be16(p + 1) }
        guard f.count >= 10 else { throw Failure(description: "quadro Cinepak curto") }
        let flags = f[0], strips = min(be16(8), Self.maxStrips)
        var p = 10, y0 = 0
        for strip in 0..<strips {
            guard p + 12 <= f.count else { throw Failure(description: "faixa Cinepak truncada") }
            var size = be24(p + 1) - 12
            guard size >= 0 else { throw Failure(description: "faixa Cinepak inválida") }
            let y1 = y0, y2 = y0 + be16(p + 8)
            p += 12
            size = min(size, f.count - p)
            if strip > 0 && flags & 1 == 0 {
                let from = (strip - 1) * Self.book, to = strip * Self.book
                for i in 0..<Self.book { v4[to + i] = v4[from + i]; v1[to + i] = v1[from + i] }
            }
            try decodeStrip(strip, y1, y2, f, p, p + size)
            p += size; y0 = y2
        }
    }

    private func decodeStrip(_ strip: Int, _ y1: Int, _ y2: Int, _ f: UnsafeBufferPointer<UInt8>, _ start: Int, _ end: Int) throws {
        var p = start
        while p + 4 <= end {
            let id = Int(f[p]), size = max(0, min((Int(f[p + 1]) << 16 | Int(f[p + 2]) << 8 | Int(f[p + 3])) - 4, end - p - 4))
            p += 4
            switch id {
            case 0x20, 0x21, 0x24, 0x25: codebook(&v4, strip, id, f, p, p + size)
            case 0x22, 0x23, 0x26, 0x27: codebook(&v1, strip, id, f, p, p + size)
            case 0x30, 0x31, 0x32: try vectors(strip, id, y1, y2, f, p, p + size); return
            default: break
            }
            p += size
        }
    }

    private func codebook(_ book: inout [UInt8], _ strip: Int, _ id: Int, _ f: UnsafeBufferPointer<UInt8>, _ start: Int, _ end: Int) {
        let n = id & 4 != 0 ? 4 : 6, partial = id & 1 != 0
        var p = start, flag: UInt32 = 0, mask: UInt32 = 0
        for entry in 0..<256 {
            if partial {
                mask >>= 1
                if mask == 0 {
                    guard p + 4 <= end else { return }
                    flag = UInt32(f[p]) << 24 | UInt32(f[p + 1]) << 16 | UInt32(f[p + 2]) << 8 | UInt32(f[p + 3]); p += 4
                    mask = 0x8000_0000
                }
                if flag & mask == 0 { continue }
            }
            guard p + n <= end else { return }
            let base = strip * Self.book + entry * 16
            let u = n == 6 ? Int(Int8(bitPattern: f[p + 4])) : 0, v = n == 6 ? Int(Int8(bitPattern: f[p + 5])) : 0
            for k in 0..<4 {
                let y = Int(f[p + k])
                book[base + 4 * k] = UInt8(clamping: y + v * 2)
                book[base + 4 * k + 1] = UInt8(clamping: y - u / 2 - v)
                book[base + 4 * k + 2] = UInt8(clamping: y + u * 2)
                book[base + 4 * k + 3] = 255
            }
            p += n
        }
    }

    private func vectors(_ strip: Int, _ id: Int, _ y1: Int, _ y2: Int, _ f: UnsafeBufferPointer<UInt8>, _ start: Int, _ end: Int) throws {
        var p = start, flag: UInt32 = 0, mask: UInt32 = 0
        let stride4 = width * 4, cb = strip * Self.book
        func read() throws {
            guard p + 4 <= end else { throw Failure(description: "vetores Cinepak truncados") }
            flag = UInt32(f[p]) << 24 | UInt32(f[p + 1]) << 16 | UInt32(f[p + 2]) << 8 | UInt32(f[p + 3]); p += 4
            mask = 0x8000_0000
        }
        try pixels.withUnsafeMutableBufferPointer { out in
            try v1.withUnsafeBufferPointer { v1 in
                try v4.withUnsafeBufferPointer { v4 in
                    // Copy one RGBX pixel of a codebook into the 4x4 block at (x, y).
                    func put(_ book: UnsafeBufferPointer<UInt8>, _ from: Int, _ x: Int, _ y: Int) {
                        guard y < height else { return }
                        let o = y * stride4 + x * 4
                        out[o] = book[from]; out[o + 1] = book[from + 1]; out[o + 2] = book[from + 2]
                    }
                    var y = y1
                    while y < y2 {
                        var x = 0
                        while x < width {
                            var coded = true
                            if id & 1 != 0 {
                                mask >>= 1; if mask == 0 { try read() }
                                coded = flag & mask != 0
                            }
                            if coded {
                                var single = id & 2 != 0
                                if !single {
                                    mask >>= 1; if mask == 0 { try read() }
                                    single = flag & mask == 0
                                }
                                if single {
                                    guard p < end else { throw Failure(description: "vetor V1 truncado") }
                                    let e = cb + Int(f[p]) * 16; p += 1
                                    for k in 0..<4 {
                                        let bx = x + (k & 1) * 2, by = y + (k >> 1) * 2
                                        for d in 0..<4 { put(v1, e + 4 * k, bx + (d & 1), by + (d >> 1)) }
                                    }
                                } else {
                                    guard p + 4 <= end else { throw Failure(description: "vetor V4 truncado") }
                                    for k in 0..<4 {
                                        let e = cb + Int(f[p + k]) * 16
                                        let bx = x + (k & 1) * 2, by = y + (k >> 1) * 2
                                        for d in 0..<4 { put(v4, e + 4 * d, bx + (d & 1), by + (d >> 1)) }
                                    }
                                    p += 4
                                }
                            }
                            x += 4
                        }
                        y += 4
                    }
                }
            }
        }
    }

    func image() -> CGImage? {
        guard let provider = CGDataProvider(data: Data(pixels) as CFData) else { return nil }
        return CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: width * 4,
                       space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                       provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
    }
}

/// Host playback of an original movie request, after PlayVideo 0x47c82c:
/// MCI open fails -> PlayVideo returns 0 (here: unavailable). Otherwise the movie
/// plays to its end, or until a WM_KEYUP of VK_ESCAPE (0x47c96e), then closes and
/// returns 1 (here: finished). Mouse input does not end it.
final class NativeDefeatMovie {
    enum Completion { case unavailable, finished }
    private var layer: CALayer?
    private var audio: AVAudioPlayer?
    private var timer: DispatchSourceTimer?
    private var completion: ((Completion, String) -> Void)?
    private var generation: UInt64 = 0
    /// Decoded pictures put on screen by the current or last movie (main thread).
    private(set) var picturesPresented = 0
    private let queue = DispatchQueue(label: "rising.original-movie", qos: .userInteractive)

    func play(_ url: URL, over view: NSView, completion: @escaping (Completion, String) -> Void) {
        cancel()
        self.completion = completion
        generation &+= 1
        let run = generation
        guard FileManager.default.fileExists(atPath: url.path) else { finish(.unavailable, "Filme original ausente"); return }
        queue.async { [weak self] in
            let prepared: (NativeAVIMovie, Data?)
            do {
                let movie = try NativeAVIMovie(data: Data(contentsOf: url, options: .alwaysMapped))
                prepared = (movie, try movie.wav())
            } catch {
                DispatchQueue.main.async { guard let self, self.generation == run else { return }
                    self.finish(.unavailable, "AVI original não decodificado: \(error)") }
                return
            }
            DispatchQueue.main.async { self?.start(prepared.0, wav: prepared.1, over: view, run: run) }
        }
    }

    /// The original's Escape key-up: MCI close, the same return as the movie's end.
    func skip() {
        guard completion != nil, layer != nil else { return }
        finish(.finished, "Filme original interrompido com Esc")
    }

    private func start(_ movie: NativeAVIMovie, wav: Data?, over view: NSView, run: UInt64) {
        guard generation == run, completion != nil else { return }
        let layer = CALayer()
        layer.frame = view.bounds; layer.autoresizingMask = [.layerWidthSizable, .layerHeightSizable]
        layer.contentsGravity = .resizeAspect; layer.backgroundColor = NSColor.black.cgColor
        view.wantsLayer = true; view.layer?.addSublayer(layer)
        self.layer = layer; picturesPresented = 0
        var duration = movie.videoDuration
        if let wav, let player = try? AVAudioPlayer(data: wav) {
            audio = player; duration = max(duration, player.duration); player.prepareToPlay(); player.play()
        }
        let decoder = NativeCinepakDecoder(width: movie.width, height: movie.height)
        let started = CACurrentMediaTime()
        var decoded = 0
        let timer = DispatchSource.makeTimerSource(queue: queue)
        timer.schedule(deadline: .now(), repeating: .milliseconds(8), leeway: .milliseconds(2))
        timer.setEventHandler { [weak self] in
            let elapsed = CACurrentMediaTime() - started
            let target = min(movie.frameCount, Int(elapsed * 1_000_000 / Double(movie.microsecondsPerFrame)) + 1)
            var failure: String?
            let advanced = decoded < target
            while decoded < target {
                do { try decoder.decode(movie.data[movie.videoChunks[decoded]]) } catch { failure = "\(error)"; break }
                decoded += 1
            }
            let image = advanced && failure == nil ? decoder.image() : nil
            let done = failure != nil || elapsed >= duration
            DispatchQueue.main.async {
                guard let self, self.generation == run else { return }
                if let image { CATransaction.begin(); CATransaction.setDisableActions(true); self.layer?.contents = image; CATransaction.commit(); self.picturesPresented += 1 }
                if let failure { self.finish(.unavailable, "Falha ao decodificar o AVI original: \(failure)") }
                else if done { self.finish(.finished, "Filme original concluído") }
            }
        }
        self.timer = timer
        timer.resume()
    }

    private func finish(_ result: Completion, _ detail: String) {
        let callback = completion; cancel(); callback?(result, detail)
    }

    func cancel() {
        completion = nil; generation &+= 1
        timer?.cancel(); timer = nil
        audio?.stop(); audio = nil
        layer?.removeFromSuperlayer(); layer = nil
    }

    deinit { timer?.cancel(); audio?.stop() }
}
