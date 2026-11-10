import Foundation
import CryptoKit

/// First-launch import of the player's own original CD. The player edition
/// ships no CD data: every original file, and every file derived from one, is
/// produced here from the image the player downloaded (the Internet Archive
/// BIN/CUE, an ISO, or a folder holding the CD's files), checked against the
/// SHA-256 values of the bundled manifest, and written to Application Support.
///
/// Output kinds (manifest "outputs"):
///   copy           an original file, byte for byte
///   ima-adpcm-wav  one original sound sample (IMA ADPCM, imaadpcmDecode4Bit_M16
///                  0x48b8c9) as a PCM16 WAV
///   pcm-wav        one uncompressed original sound sample as a WAV
///   cd-audio       a CD audio track of the Brazilian BIN (NativeCDMusicDisc)
///   keyed          a runtime-derived table shipped XOR a keystream that only the
///                  CD's own bytes (keySources) produce; meaningless without the CD
///   bundled        app data that holds no CD content (hash manifests, host inputs)
enum NativeCDImportError: LocalizedError, Equatable {
    case unrecognized(String)
    case missing(String)
    case mismatch(String)
    case manifest(String)
    case io(String)

    var errorDescription: String? {
        switch self {
        case .unrecognized(let name):
            return "“\(name)” não parece ser o CD do Rising Lands. Escolha a imagem RISING_LANDS_BR_.BIN (ou o arquivo .CUE ou .ISO) ou a pasta com os arquivos do CD."
        case .missing(let path):
            return "O arquivo \(path) não foi encontrado no CD escolhido."
        case .mismatch(let path):
            return "O arquivo \(path) do CD escolhido é diferente do CD brasileiro original (verificação SHA-256). Use a imagem original, sem modificações."
        case .manifest(let detail):
            return "O manifesto de importação deste aplicativo é inválido (\(detail)). Baixe o aplicativo novamente."
        case .io(let detail):
            return "Não foi possível gravar os dados importados: \(detail)"
        }
    }
}

protocol NativeCDImportSource {
    var label: String { get }
    /// The raw 2,352-byte-sector image, when the source is one (CD audio needs it).
    var rawImage: URL? { get }
    func file(_ path: String) throws -> Data
}

func nativeSHA256(_ data: Data) -> String { SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined() }

/// ISO 9660 reader over a raw MODE1/MODE2 2,352-byte image or a 2,048-byte ISO.
final class NativeCDImage: NativeCDImportSource {
    let url: URL
    let sectorSize: UInt64
    let dataOffset: UInt64
    private let handle: FileHandle
    private let root: (lba: UInt32, length: UInt32)
    var label: String { url.lastPathComponent }
    var rawImage: URL? { sectorSize == 2352 ? url : nil }

    init(url: URL) throws {
        self.url = url
        guard let handle = try? FileHandle(forReadingFrom: url) else { throw NativeCDImportError.unrecognized(url.lastPathComponent) }
        self.handle = handle
        let size = (try? handle.seekToEnd()) ?? 0
        func bytes(_ offset: UInt64, _ count: Int) -> Data? {
            guard offset + UInt64(count) <= size, (try? handle.seek(toOffset: offset)) != nil else { return nil }
            return try? handle.read(upToCount: count)
        }
        // Mode 2 form 1 (24-byte header), mode 1 (16) and plain ISO (0).
        let layouts: [(UInt64, UInt64)] = [(2352, 24), (2352, 16), (2048, 0)]
        guard let layout = layouts.first(where: { bytes(16 * $0.0 + $0.1 + 1, 5) == Data("CD001".utf8) && bytes(16 * $0.0 + $0.1, 1) == Data([1]) }),
              let pvd = bytes(16 * layout.0 + layout.1, 2048) else {
            throw NativeCDImportError.unrecognized(url.lastPathComponent)
        }
        sectorSize = layout.0; dataOffset = layout.1
        root = (Self.le32(pvd, 156 + 2), Self.le32(pvd, 156 + 10))
    }
    deinit { try? handle.close() }

    static func le32(_ data: Data, _ offset: Int) -> UInt32 {
        let base = data.startIndex + offset
        return UInt32(data[base]) | UInt32(data[base + 1]) << 8 | UInt32(data[base + 2]) << 16 | UInt32(data[base + 3]) << 24
    }

    func extent(_ lba: UInt32, _ length: UInt32) throws -> Data {
        let sectors = (UInt64(length) + 2047) / 2048
        guard (try? handle.seek(toOffset: UInt64(lba) * sectorSize)) != nil,
              let raw = try handle.read(upToCount: Int(sectors * sectorSize)), UInt64(raw.count) == sectors * sectorSize else {
            throw NativeCDImportError.unrecognized(label)
        }
        if sectorSize == 2048 { return raw.prefix(Int(length)) }
        var out = Data(capacity: Int(sectors) * 2048)
        for index in 0..<Int(sectors) {
            let start = raw.startIndex + index * Int(sectorSize) + Int(dataOffset)
            out.append(raw[start..<start + 2048])
        }
        return out.prefix(Int(length))
    }

    static func normalized(_ name: String) -> String {
        var value = name.uppercased()
        if let semicolon = value.firstIndex(of: ";") { value = String(value[..<semicolon]) }
        while value.hasSuffix(".") { value.removeLast() }
        return value
    }

    private func entries(_ lba: UInt32, _ length: UInt32) throws -> [String: (UInt32, UInt32, Bool)] {
        let data = try extent(lba, length)
        var result = [String: (UInt32, UInt32, Bool)]()
        var offset = 0
        while offset < data.count {
            let size = Int(data[data.startIndex + offset])
            if size == 0 { offset = (offset / 2048 + 1) * 2048; continue }
            guard offset + size <= data.count, size >= 34 else { break }
            let record = data.subdata(in: data.startIndex + offset..<data.startIndex + offset + size)
            let nameLength = Int(record[32])
            if nameLength > 0, 33 + nameLength <= size {
                let raw = record.subdata(in: 33..<33 + nameLength)
                if raw != Data([0]) && raw != Data([1]), let name = String(data: raw, encoding: .ascii) {
                    result[Self.normalized(name)] = (Self.le32(record, 2), Self.le32(record, 10), record[25] & 2 != 0)
                }
            }
            offset += size
        }
        return result
    }

    func file(_ path: String) throws -> Data {
        var current = (root.lba, root.length, true)
        for component in path.split(separator: "/").map(String.init) {
            guard current.2, let next = try entries(current.0, current.1)[Self.normalized(component)] else {
                throw NativeCDImportError.missing(path)
            }
            current = next
        }
        guard !current.2 else { throw NativeCDImportError.missing(path) }
        return try extent(current.0, current.1)
    }
}

/// A folder holding the CD's files (a mounted disc or an extracted copy).
final class NativeCDFolder: NativeCDImportSource {
    let root: URL
    var label: String { root.lastPathComponent }
    var rawImage: URL? { nil }

    init(url: URL) throws {
        root = url
        guard (try? Self.resolve(url, "INSTALHD")) != nil else { throw NativeCDImportError.unrecognized(url.lastPathComponent) }
    }

    static func resolve(_ base: URL, _ path: String) throws -> URL {
        var current = base
        for component in path.split(separator: "/").map(String.init) {
            let names = (try? FileManager.default.contentsOfDirectory(atPath: current.path)) ?? []
            guard let match = names.first(where: { NativeCDImage.normalized($0) == NativeCDImage.normalized(component) }) else {
                throw NativeCDImportError.missing(path)
            }
            current = current.appendingPathComponent(match)
        }
        return current
    }

    func file(_ path: String) throws -> Data {
        let url = try Self.resolve(root, path)
        guard let data = try? Data(contentsOf: url) else { throw NativeCDImportError.missing(path) }
        return data
    }
}

enum NativeCDImport {
    static let schema = "rising-lands/player-import/1"
    static let keyDomain = "RisingLands-player-import-v1"

    /// Opens what the player chose: a .bin/.iso image, a .cue sheet or a folder.
    static func source(_ url: URL) throws -> NativeCDImportSource {
        var directory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: url.path, isDirectory: &directory) else {
            throw NativeCDImportError.unrecognized(url.lastPathComponent)
        }
        if directory.boolValue { return try NativeCDFolder(url: url) }
        if url.pathExtension.lowercased() == "cue" {
            guard let text = try? String(contentsOf: url, encoding: .isoLatin1),
                  let line = text.split(whereSeparator: \.isNewline).first(where: { $0.trimmingCharacters(in: .whitespaces).uppercased().hasPrefix("FILE ") }),
                  let first = line.firstIndex(of: "\""), let last = line.lastIndex(of: "\""), first < last else {
                throw NativeCDImportError.unrecognized(url.lastPathComponent)
            }
            let name = String(line[line.index(after: first)..<last])
            return try NativeCDImage(url: url.deletingLastPathComponent().appendingPathComponent(name))
        }
        return try NativeCDImage(url: url)
    }

    /// imaadpcmDecode4Bit_M16 0x48b8c9: blocks of min(remaining, block) bytes while
    /// at least 4 remain; header i16 predictor, u8 step index (above 88 stops),
    /// then low and high nibble of each byte. Returns little-endian PCM16 bytes.
    static func imaMono16(_ source: [UInt8], block: Int) -> [UInt8]? {
        let indexTable = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8]
        let steps = [7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73,
                     80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494,
                     544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499,
                     2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487,
                     12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767]
        var remaining = source.count, position = 0
        var out = [UInt8](); out.reserveCapacity(source.count * 4 + 4)
        func emit(_ value: Int) { let v = UInt16(bitPattern: Int16(value)); out.append(UInt8(v & 0xff)); out.append(UInt8(v >> 8)) }
        if remaining >= 4 && block < 4 { return nil }
        while remaining >= 4 {
            let count = min(remaining, block)
            remaining -= count
            var predictor = Int(Int16(bitPattern: UInt16(source[position]) | UInt16(source[position + 1]) << 8))
            var index = Int(source[position + 2])
            position += 4
            if index > 88 { return out }
            emit(predictor)
            for byte in source[position..<position + count - 4] {
                for nibble in [Int(byte & 15), Int(byte >> 4)] {
                    let step = steps[index]
                    var difference = step >> 3
                    if nibble & 4 != 0 { difference += step }
                    if nibble & 2 != 0 { difference += step >> 1 }
                    if nibble & 1 != 0 { difference += step >> 2 }
                    predictor = max(-32768, min(32767, predictor + (nibble & 8 != 0 ? -difference : difference)))
                    index = min(88, max(0, index + indexTable[nibble]))
                    emit(predictor)
                }
            }
            position += count - 4
        }
        return out
    }

    static func wav(_ pcm: [UInt8], channels: Int, rate: Int, bits: Int) -> Data {
        var data = Data("RIFF".utf8)
        func word(_ value: Int, _ bytes: Int = 4) { for i in 0..<bytes { data.append(UInt8(truncatingIfNeeded: value >> (i * 8))) } }
        let align = channels * bits / 8
        word(pcm.count + 36); data.append(Data("WAVEfmt ".utf8)); word(16)
        word(1, 2); word(channels, 2); word(rate); word(rate * align); word(align, 2); word(bits, 2)
        data.append(Data("data".utf8)); word(pcm.count); data.append(contentsOf: pcm)
        return data
    }

    /// The keystream of a keyed output: SHA-256(K ‖ counter) blocks, with
    /// K = SHA-256(domain ‖ 0 ‖ path ‖ 0 ‖ key source bytes…).
    static func keyed(_ blob: Data, path: String, keySources: [Data]) -> Data {
        var hasher = SHA256()
        hasher.update(data: Data(keyDomain.utf8)); hasher.update(data: Data([0]))
        hasher.update(data: Data(path.utf8)); hasher.update(data: Data([0]))
        for source in keySources { hasher.update(data: source) }
        let key = Data(hasher.finalize())
        var out = [UInt8](blob)
        var counter: UInt64 = 0, offset = 0
        while offset < out.count {
            var block = key
            withUnsafeBytes(of: counter.littleEndian) { block.append(contentsOf: $0) }
            for byte in SHA256.hash(data: block) where offset < out.count { out[offset] ^= byte; offset += 1 }
            counter += 1
        }
        return Data(out)
    }

    struct Report { let written: Int; let music: Bool; let manifestID: String }

    /// Produces every output into `destination` (created; must not exist).
    static func run(manifestURL: URL, resources: URL, source: NativeCDImportSource, destination: URL,
                    progress: (Double, String) -> Void = { _, _ in }) throws -> Report {
        guard let data = try? Data(contentsOf: manifestURL),
              let manifest = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              manifest["schema"] as? String == schema, let manifestID = manifest["id"] as? String,
              let sources = manifest["sources"] as? [String: [String: Any]],
              let outputs = manifest["outputs"] as? [[String: Any]], !outputs.isEmpty else {
            throw NativeCDImportError.manifest("schema")
        }
        var cache = [String: Data]()
        func original(_ path: String) throws -> Data {
            if let data = cache[path] { return data }
            guard let expected = sources[path]?["sha256"] as? String else { throw NativeCDImportError.manifest(path) }
            let data = try source.file(path)
            guard nativeSHA256(data) == expected else { throw NativeCDImportError.mismatch(path) }
            cache[path] = data
            return data
        }
        let music = source.rawImage != nil
        // Verify every needed original first, so a wrong disc fails before writing.
        let needed = Set(outputs.flatMap { output -> [String] in
            guard (output["group"] as? String) != "music" || music else { return [] }
            return ([output["source"] as? String] + ((output["keySources"] as? [String]) ?? []).map { Optional($0) }).compactMap { $0 }
        })
        for (index, path) in needed.sorted().enumerated() {
            progress(0.4 * Double(index) / Double(max(1, needed.count)), "Verificando \(path)…")
            _ = try original(path)
        }
        do { try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: false) }
        catch { throw NativeCDImportError.io(error.localizedDescription) }
        var written = 0
        for (index, output) in outputs.enumerated() {
            guard let path = output["path"] as? String, let kind = output["kind"] as? String,
                  let expected = output["sha256"] as? String, !path.hasPrefix("/"), !path.split(separator: "/").contains("..") else {
                throw NativeCDImportError.manifest("output \(index)")
            }
            if output["group"] as? String == "music" && !music { continue }
            if index % 25 == 0 { progress(0.4 + 0.6 * Double(index) / Double(outputs.count), "Preparando \(path)…") }
            let produced: Data
            switch kind {
            case "copy":
                guard let from = output["source"] as? String else { throw NativeCDImportError.manifest(path) }
                produced = try original(from)
            case "ima-adpcm-wav", "pcm-wav":
                guard let from = output["source"] as? String, let offset = output["offset"] as? Int, let length = output["length"] as? Int,
                      let channels = output["channels"] as? Int, let rate = output["rate"] as? Int, let bits = output["bits"] as? Int else {
                    throw NativeCDImportError.manifest(path)
                }
                let bank = try original(from)
                guard offset >= 0, length >= 0, offset + length <= bank.count else { throw NativeCDImportError.manifest(path) }
                let bytes = [UInt8](bank.subdata(in: bank.startIndex + offset..<bank.startIndex + offset + length))
                if kind == "pcm-wav" {
                    let align = max(1, channels * bits / 8)
                    produced = wav(Array(bytes.prefix(bytes.count - bytes.count % align)), channels: channels, rate: rate, bits: bits)
                } else {
                    guard channels == 1, bits == 16, let block = output["block"] as? Int, let pcm = imaMono16(bytes, block: block) else {
                        throw NativeCDImportError.manifest(path)
                    }
                    produced = wav(pcm, channels: 1, rate: rate, bits: 16)
                }
            case "cd-audio":
                guard let image = source.rawImage, let track = output["track"] as? Int else { throw NativeCDImportError.manifest(path) }
                do { produced = try NativeCDMusicDisc.wav(track: track, image: image) }
                catch { throw NativeCDImportError.mismatch("faixa de áudio \(track)") }
            case "keyed":
                guard let blob = output["blob"] as? String, let keys = output["keySources"] as? [String], !keys.isEmpty,
                      let shipped = try? Data(contentsOf: resources.appendingPathComponent(blob)) else {
                    throw NativeCDImportError.manifest(path)
                }
                produced = keyed(shipped, path: path, keySources: try keys.map(original))
            case "bundled":
                guard let blob = output["blob"] as? String, let shipped = try? Data(contentsOf: resources.appendingPathComponent(blob)) else {
                    throw NativeCDImportError.manifest(path)
                }
                produced = shipped
            default:
                throw NativeCDImportError.manifest(kind)
            }
            guard nativeSHA256(produced) == expected else {
                throw kind == "copy" ? NativeCDImportError.mismatch(output["source"] as? String ?? path) : NativeCDImportError.mismatch(path)
            }
            let target = destination.appendingPathComponent(path)
            do {
                try FileManager.default.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
                try produced.write(to: target)
            } catch { throw NativeCDImportError.io(error.localizedDescription) }
            written += 1
        }
        let receipt: [String: Any] = ["schema": schema, "manifestId": manifestID, "source": source.label, "music": music,
                                      "files": written, "importedAt": ISO8601DateFormatter().string(from: Date())]
        do {
            try JSONSerialization.data(withJSONObject: receipt, options: [.prettyPrinted, .sortedKeys])
                .write(to: destination.appendingPathComponent("importacao.json"))
        } catch { throw NativeCDImportError.io(error.localizedDescription) }
        progress(1, "Importação concluída.")
        return Report(written: written, music: music, manifestID: manifestID)
    }

    /// Imports into `<folder>/Dados` atomically: a staging folder is renamed in
    /// only after every output verified; an earlier import is replaced then.
    static func install(manifestURL: URL, resources: URL, source: NativeCDImportSource, folder: URL,
                        progress: (Double, String) -> Void = { _, _ in }) throws -> Report {
        do { try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true) }
        catch { throw NativeCDImportError.io(error.localizedDescription) }
        let staging = folder.appendingPathComponent("Dados.importando-\(UUID().uuidString)")
        do {
            let report = try run(manifestURL: manifestURL, resources: resources, source: source, destination: staging, progress: progress)
            let final = folder.appendingPathComponent("Dados")
            if FileManager.default.fileExists(atPath: final.path) { try FileManager.default.removeItem(at: final) }
            try FileManager.default.moveItem(at: staging, to: final)
            return report
        } catch {
            try? FileManager.default.removeItem(at: staging)
            if let known = error as? NativeCDImportError { throw known }
            throw NativeCDImportError.io(error.localizedDescription)
        }
    }

    /// True when `<folder>/Dados` holds a complete import for this manifest.
    static func installed(manifestURL: URL, folder: URL) -> Bool {
        let data = folder.appendingPathComponent("Dados")
        guard let manifestData = try? Data(contentsOf: manifestURL),
              let manifest = try? JSONSerialization.jsonObject(with: manifestData) as? [String: Any],
              let receiptData = try? Data(contentsOf: data.appendingPathComponent("importacao.json")),
              let receipt = try? JSONSerialization.jsonObject(with: receiptData) as? [String: Any],
              receipt["manifestId"] as? String == manifest["id"] as? String,
              let outputs = manifest["outputs"] as? [[String: Any]] else { return false }
        let music = receipt["music"] as? Bool ?? false
        return outputs.allSatisfy { output in
            guard let path = output["path"] as? String, let bytes = output["bytes"] as? Int else { return false }
            if output["group"] as? String == "music" && !music { return true }
            let size = (try? FileManager.default.attributesOfItem(atPath: data.appendingPathComponent(path).path))?[.size] as? Int
            return size == bytes
        }
    }
}
