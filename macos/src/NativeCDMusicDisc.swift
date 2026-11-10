import Foundation
import CryptoKit

/// Read-only, exact Brazilian BIN windows from 358934b2. Never creates a WAV
/// on disk. Each returned WAV must match the reviewed complete WAV identity.
enum NativeCDMusicDisc {
    static let imageBytes: UInt64 = 705_418_896
    static let imageSHA256 = "c2c09e87927cf5ebbe5d17ea17bffb53c82c2e68385e9259ad0453cc15ea616f"
    static let cueSHA256 = "766fbc02932e3f0454d411211991016142fd361930fc51528fe858fcbf738873"

    static func wav(track: Int, image: URL) throws -> Data {
        let offset: UInt64, count: Int, expected: String
        switch track {
        case 2:
            offset = 534_569_616; count = 99_654_240
            expected = "bee34ef8aef0a58933f90d63766eafc3f8daf447aae64ea35157d84c6850596a"
        case 3:
            offset = 634_223_856; count = 71_195_040
            expected = "80b28a77ea82e454aa6babeb788746d2a141a932f4d101d7c4a8ecc51870ae3d"
        default: throw NativeCDMusicError.unsupportedTrack(track)
        }
        guard image.isFileURL, image.query == nil, image.fragment == nil,
              image.host == nil || image.host == "" || image.host == "localhost" else {
            throw NativeCDMusicError.invalidOriginal(track)
        }
        do {
            let file = try FileHandle(forReadingFrom: image)
            defer { try? file.close() }
            guard try file.seekToEnd() == imageBytes else { throw NativeCDMusicError.invalidOriginal(track) }
            try file.seek(toOffset: offset)
            var wav = Data("RIFF".utf8)
            func word(_ value: UInt32, _ bytes: Int = 4) {
                for i in 0..<bytes { wav.append(UInt8(truncatingIfNeeded: value >> (i * 8))) }
            }
            word(UInt32(count + 36)); wav.append(Data("WAVEfmt ".utf8)); word(16)
            word(1, 2); word(2, 2); word(44_100); word(176_400); word(4, 2); word(16, 2)
            wav.append(Data("data".utf8)); word(UInt32(count)); wav.reserveCapacity(count + 44)
            while wav.count < count + 44 {
                let data = try file.read(upToCount: min(1_048_576, count + 44 - wav.count)) ?? Data()
                guard !data.isEmpty else { throw NativeCDMusicError.invalidOriginal(track) }
                wav.append(data)
            }
            guard SHA256.hash(data: wav).map({ String(format: "%02x", $0) }).joined() == expected else {
                throw NativeCDMusicError.invalidOriginal(track)
            }
            return wav
        } catch let error as NativeCDMusicError { throw error }
        catch { throw NativeCDMusicError.missingOriginal(track) }
    }
}
