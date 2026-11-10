import Foundation

/// Autodesk FLC/FLI decoder for the original in-game event animations
/// (`Events\NAME.FLC`, lane B's event contract motor/include/event-flc.md).
/// Host presentation only, as the original's FlicOpen/FlicPlay layer: an 8-bit
/// frame buffer plus a 256-colour palette, advanced one frame chunk at a time.
/// Chunks: COLOR_256 (4), COLOR_64 (11), BLACK (13), BRUN (15), COPY (16),
/// LC (12, FLI delta) and SS2 (7, FLC word delta); others (PSTAMP 18, ...) skip.
struct NativeFLCError: Error, CustomStringConvertible {
    let description: String
}

final class NativeFLC {
    let data: [UInt8]
    let frames: Int, width: Int, height: Int
    /// Milliseconds per frame (FLI speed is in 1/70 s and is converted).
    let delayMS: Double
    private(set) var pixels: [UInt8]
    /// 256 × RGB, 8 bits per component.
    private(set) var palette: [UInt8]
    private(set) var paletteChanged = false
    /// Frame chunks decoded so far (1 after the first frame).
    private(set) var shown = 0
    private var offset: Int
    private let firstFrame: Int

    init(_ bytes: [UInt8]) throws {
        data = bytes
        guard bytes.count >= 128 else { throw NativeFLCError(description: "FLC header truncated") }
        func u16(_ o: Int) -> Int { Int(bytes[o]) | Int(bytes[o + 1]) << 8 }
        func u32(_ o: Int) -> Int { u16(o) | u16(o + 2) << 16 }
        let magic = u16(4)
        guard magic == 0xAF12 || magic == 0xAF11 else { throw NativeFLCError(description: String(format: "not an FLC/FLI (magic %04x)", magic)) }
        frames = u16(6); width = u16(8); height = u16(10)
        guard frames > 0, width > 0, height > 0, width <= 800, height <= 600, u16(12) == 8 || u16(12) == 0 else {
            throw NativeFLCError(description: "unsupported FLC geometry")
        }
        let speed = u32(16)
        delayMS = magic == 0xAF11 ? Double(speed) * 1000 / 70 : Double(speed)
        let frame1 = magic == 0xAF12 ? u32(80) : 0
        firstFrame = frame1 >= 128 && frame1 < bytes.count ? frame1 : 128
        offset = firstFrame
        pixels = [UInt8](repeating: 0, count: width * height)
        palette = [UInt8](repeating: 0, count: 768)
    }

    convenience init(contentsOf url: URL) throws {
        try self.init([UInt8](try Data(contentsOf: url)))
    }

    /// Decodes the next frame chunk (prefix chunks 0xF100 are skipped).
    /// After `frames` chunks, a file's ring frame (if present) returns to frame 1.
    func next() throws {
        paletteChanged = false
        while true {
            guard offset + 16 <= data.count else { throw NativeFLCError(description: "frame \(shown + 1) beyond end of file") }
            let size = u32(offset), type = u16(offset + 4)
            guard size >= 16, offset + size <= data.count else { throw NativeFLCError(description: "frame \(shown + 1) size \(size) invalid") }
            if type == 0xF100 { offset += size; continue }
            guard type == 0xF1FA else { throw NativeFLCError(description: String(format: "frame %d chunk type %04x", shown + 1, type)) }
            let count = u16(offset + 6)
            var chunk = offset + 16
            for _ in 0..<count {
                guard chunk + 6 <= offset + size else { throw NativeFLCError(description: "frame \(shown + 1) chunk header beyond frame") }
                var length = u32(chunk)
                let kind = u16(chunk + 4)
                // Some writers store an odd size and pad; never step past the frame.
                if length < 6 || chunk + length > offset + size {
                    guard length >= 6, chunk + length - 1 <= offset + size else { throw NativeFLCError(description: "frame \(shown + 1) chunk \(kind) size \(length)") }
                    length = offset + size - chunk
                }
                // The original event files' COPY chunks declare 2 bytes less than
                // their 6 + width×height (the frame size is right): read to the frame.
                if kind == 16 && length < 6 + width * height { length = min(6 + width * height, offset + size - chunk) }
                try decode(kind, chunk + 6, chunk + length)
                chunk += length
            }
            offset += size
            shown += 1
            return
        }
    }

    /// Back to frame 1's position (the buffer keeps the last image, as a loop's ring frame expects).
    func rewind() { offset = firstFrame; shown = 0 }

    private func u16(_ o: Int) -> Int { Int(data[o]) | Int(data[o + 1]) << 8 }
    private func u32(_ o: Int) -> Int { u16(o) | u16(o + 2) << 16 }

    private func decode(_ kind: Int, _ start: Int, _ end: Int) throws {
        func fail(_ what: String) -> NativeFLCError { NativeFLCError(description: "frame \(shown + 1) chunk \(kind): \(what)") }
        var p = start
        func byte() throws -> Int { guard p < end else { throw fail("truncated") }; defer { p += 1 }; return Int(data[p]) }
        func word() throws -> Int { let lo = try byte(); return lo | (try byte()) << 8 }
        switch kind {
        case 4, 11:
            let packets = try word()
            var index = 0
            for _ in 0..<packets {
                index += try byte()
                var count = try byte(); if count == 0 { count = 256 }
                for _ in 0..<count {
                    guard index < 256 else { throw fail("palette overflow") }
                    for c in 0..<3 {
                        let v = try byte()
                        palette[index * 3 + c] = kind == 4 ? UInt8(v) : UInt8(min(255, (v & 63) << 2 | (v & 63) >> 4))
                    }
                    index += 1
                }
            }
            paletteChanged = true
        case 13:
            for i in pixels.indices { pixels[i] = 0 }
        case 16:
            guard end - start >= width * height else { throw fail("COPY truncated") }
            for i in 0..<(width * height) { pixels[i] = data[start + i] }
        case 15:
            for y in 0..<height {
                _ = try byte() // packet count: ignored, the line is filled by width
                var x = 0
                while x < width {
                    let count = Int(Int8(bitPattern: UInt8(try byte())))
                    if count >= 0 {
                        let value = UInt8(try byte())
                        guard x + count <= width else { throw fail("BRUN run past line") }
                        for i in 0..<count { pixels[y * width + x + i] = value }
                        x += count
                    } else {
                        guard x - count <= width, p - count <= end else { throw fail("BRUN copy past line") }
                        for i in 0..<(-count) { pixels[y * width + x + i] = data[p + i] }
                        p -= count; x -= count
                    }
                    if count == 0 { break }
                }
            }
        case 12:
            var y = try word()
            let lines = try word()
            for _ in 0..<lines {
                guard y < height else { throw fail("LC line past image") }
                var x = 0
                let packets = try byte()
                for _ in 0..<packets {
                    x += try byte()
                    let count = Int(Int8(bitPattern: UInt8(try byte())))
                    if count >= 0 {
                        guard x + count <= width, p + count <= end else { throw fail("LC copy past line") }
                        for i in 0..<count { pixels[y * width + x + i] = data[p + i] }
                        p += count; x += count
                    } else {
                        let value = UInt8(try byte())
                        guard x - count <= width else { throw fail("LC run past line") }
                        for i in 0..<(-count) { pixels[y * width + x + i] = value }
                        x -= count
                    }
                }
                y += 1
            }
        case 7:
            let lines = try word()
            var y = 0
            for _ in 0..<lines {
                var packets = 0
                while true {
                    let op = try word()
                    switch op >> 14 {
                    case 0: packets = op
                    case 3: y += 65536 - op // signed negative line skip
                        continue
                    case 2:
                        guard y < height else { throw fail("SS2 last byte past image") }
                        pixels[y * width + width - 1] = UInt8(op & 0xff)
                        continue
                    default: throw fail("SS2 opcode")
                    }
                    break
                }
                guard y < height else { throw fail("SS2 line past image") }
                var x = 0
                for _ in 0..<packets {
                    x += try byte()
                    let count = Int(Int8(bitPattern: UInt8(try byte())))
                    if count >= 0 {
                        guard x + 2 * count <= width, p + 2 * count <= end else { throw fail("SS2 copy past line") }
                        for i in 0..<(2 * count) { pixels[y * width + x + i] = data[p + i] }
                        p += 2 * count; x += 2 * count
                    } else {
                        let lo = UInt8(try byte()), hi = UInt8(try byte())
                        guard x - 2 * count <= width else { throw fail("SS2 run past line") }
                        for i in 0..<(-count) { pixels[y * width + x + 2 * i] = lo; pixels[y * width + x + 2 * i + 1] = hi }
                        x -= 2 * count
                    }
                }
                y += 1
            }
        default:
            break // PSTAMP and other chunks carry no image data for playback.
        }
    }

    /// Overlay the current frame centred on an 800×600 indexed game surface.
    func composite(into surface: inout [UInt8]) {
        let left = (800 - width) / 2, top = (600 - height) / 2
        for y in 0..<height {
            let row = (top + y) * 800 + left
            for x in 0..<width { surface[row + x] = pixels[y * width + x] }
        }
    }
}
