import AVFoundation
import CryptoKit
import Foundation

enum NativeCDMusicError: Error, LocalizedError, Equatable {
    case unsupportedTrack(Int)
    case invalidVolume
    case missingOriginal(Int)
    case invalidOriginal(Int)
    case playbackFailed(Int)
    case closed

    var errorDescription: String? {
        switch self {
        case .unsupportedTrack(let track):
            return "Faixa de CD não suportada: \(track). Use a faixa original 2 ou 3."
        case .invalidVolume:
            return "Volume da música inválido. Use um valor entre 0 e 1."
        case .missingOriginal(let track):
            return "Arquivo de música original da faixa \(track) ausente ou inacessível."
        case .invalidOriginal(let track):
            return "Arquivo de música original da faixa \(track) inválido. Verifique o WAV original do CD brasileiro."
        case .playbackFailed(let track):
            return "Não foi possível reproduzir a faixa original \(track) do CD."
        case .closed:
            return "O reprodutor de música do CD foi encerrado."
        }
    }
}

enum NativeCDMusicCompletion {
    case ended
    case failed
}

/// One non-looping stream. All methods and completion delivery use the main thread.
/// stop must release device resources and must tolerate repeated calls.
@MainActor protocol NativeCDMusicBackend: AnyObject {
    var volume: Float { get set }
    var onCompletion: ((NativeCDMusicCompletion) -> Void)? { get set }
    func play() throws
    func stop()
}

/// Host presentation only: no engine state, selection, RNG, timers or preferences.
/// Construct, command, observe and release on the main thread. The factory must
/// only prepare a stream; play is the sole operation permitted to start sound.
@MainActor final class NativeCDMusicPlayer {
    enum Command {
        case play(track: Int)
        case stop
        case setVolume(Float)
        case setMuted(Bool)
        case close
    }

    enum State: Equatable {
        case idle
        case playing(track: Int)
        case closed
    }

    enum Event: Equatable {
        case started(track: Int)
        case stopped
        case finished(track: Int)
        case failed(NativeCDMusicError)
        case closed
    }

    typealias BackendFactory = @MainActor (URL) throws -> NativeCDMusicBackend

    private(set) var state: State = .idle
    private(set) var volume: Float
    private(set) var isMuted: Bool
    /// Delivered synchronously after state/resource updates; observers may issue commands.
    /// Synchronous command failures throw; asynchronous playback failures emit .failed.
    var onEvent: ((Event) -> Void)?
    var deviceEvidence: [String: Any] { (backend as? NativeCDMusicAVBackend)?.deviceEvidence ?? [:] }

    private let prepareBackend: @MainActor (Int) throws -> NativeCDMusicBackend
    private var backend: NativeCDMusicBackend?
    private var generation: UInt64 = 0

    init(originalTrackURLs: [Int: URL], volume: Float = 1, muted: Bool = false,
         backendFactory: @escaping BackendFactory = { try NativeCDMusicAVBackend(url: $0) }) throws {
        precondition(Thread.isMainThread)
        guard volume.isFinite, (0...1).contains(volume) else { throw NativeCDMusicError.invalidVolume }
        // Partial catalogs are allowed so a missing original has a track-specific error.
        for track in originalTrackURLs.keys.sorted() where track != 2 && track != 3 {
            throw NativeCDMusicError.unsupportedTrack(track)
        }
        NativeCDMusicOriginal.prewarm(originalTrackURLs)
        self.volume = volume
        self.isMuted = muted
        self.prepareBackend = { track in
            let url = try NativeCDMusicOriginal.validate(track: track, url: originalTrackURLs[track])
            do { return try backendFactory(url) }
            catch { throw NativeCDMusicError.playbackFailed(track) }
        }
    }

    /// The Mini already has the verified disc image. Read an exact track into
    /// memory rather than extracting/copying another commercial-media file.
    init(originalDiscURL: URL,
         backendFactory: @escaping @MainActor (Int, Data) throws -> NativeCDMusicBackend = {
             _, data in try NativeCDMusicAVBackend(data: data)
         }) {
        precondition(Thread.isMainThread)
        volume = 1
        isMuted = false
        prepareBackend = { track in
            let data = try NativeCDMusicDisc.wav(track: track, image: originalDiscURL)
            do { return try backendFactory(track, data) }
            catch { throw NativeCDMusicError.playbackFailed(track) }
        }
    }

    isolated deinit {
        backend?.onCompletion = nil
        backend?.stop()
    }

    func execute(_ command: Command) throws {
        precondition(Thread.isMainThread)
        if case .close = command {
            guard state != .closed else { return }
            releaseStream()
            state = .closed
            let observer = onEvent
            onEvent = nil
            observer?(.closed)
            return
        }
        guard state != .closed else { throw NativeCDMusicError.closed }
        switch command {
        case .play(let track):
            try play(track: track)
        case .stop:
            releaseStream()
            onEvent?(.stopped)
        case .setVolume(let value):
            guard value.isFinite, (0...1).contains(value) else { throw NativeCDMusicError.invalidVolume }
            volume = value
            backend?.volume = isMuted ? 0 : volume
        case .setMuted(let value):
            isMuted = value
            backend?.volume = isMuted ? 0 : volume
        case .close:
            break // Handled above, including repeated close.
        }
    }

    private func play(track: Int) throws {
        // Validate before disrupting the current stream. Never substitute another track.
        let next = try prepareBackend(track)
        next.volume = isMuted ? 0 : volume
        releaseStream()
        let request = generation
        backend = next
        state = .playing(track: track)
        next.onCompletion = { [weak self] completion in
            self?.complete(completion, track: track, request: request)
        }
        do { try next.play() }
        catch {
            if generation == request { releaseStream() }
            throw NativeCDMusicError.playbackFailed(track)
        }
        // A backend may complete synchronously. Do not announce a stale start.
        if generation == request, backend != nil { onEvent?(.started(track: track)) }
    }

    private func complete(_ completion: NativeCDMusicCompletion, track: Int, request: UInt64) {
        precondition(Thread.isMainThread)
        guard generation == request, backend != nil else { return }
        releaseStream()
        switch completion {
        case .ended: onEvent?(.finished(track: track))
        case .failed: onEvent?(.failed(.playbackFailed(track)))
        }
    }

    private func releaseStream() {
        generation &+= 1
        let old = backend
        backend = nil
        state = .idle
        old?.onCompletion = nil
        old?.stop()
    }
}

/// Exact app-ready WAV identities from music-source commit 358934b2116fd9c0ff06b49617e3fe2c37be4163.
/// Hashing streams read-only, with bounded memory, before every explicit Play.
enum NativeCDMusicOriginal {
    /// Files already proven equal to their identity, keyed by track and path and
    /// bound to size, modification date and inode: the 71-100 MB hash runs once
    /// per session (prewarmed off the main thread), not on every track start,
    /// which cost the main thread 35-130 ms (lane D).
    private static var verified = [String: [AnyHashable]]()
    private static let verifiedLock = NSLock()
    private static func identity(_ url: URL) -> [AnyHashable]? {
        guard let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
              let size = attributes[.size] as? NSNumber, let date = attributes[.modificationDate] as? Date,
              let inode = attributes[.systemFileNumber] as? NSNumber else { return nil }
        return [size, date, inode]
    }

    /// Verify the catalog's originals on a background queue ahead of the first Play.
    static func prewarm(_ urls: [Int: URL]) {
        DispatchQueue.global(qos: .utility).async {
            for (track, url) in urls.sorted(by: { $0.key < $1.key }) { _ = try? validate(track: track, url: url) }
        }
    }

    static func validate(track: Int, url: URL?) throws -> URL {
        if let url = url, url.isFileURL, url.query == nil, url.fragment == nil,
           url.host == nil || url.host == "" || url.host == "localhost" {
            let key = "\(track)|\(url.standardizedFileURL.path)"
            verifiedLock.lock(); let known = verified[key]; verifiedLock.unlock()
            if let known = known, known == identity(url.standardizedFileURL) { return url.standardizedFileURL }
        }
        let checked = try validateBytes(track: track, url: url)
        if let id = identity(checked) {
            verifiedLock.lock(); verified["\(track)|\(checked.path)"] = id; verifiedLock.unlock()
        }
        return checked
    }

    private static func validateBytes(track: Int, url: URL?) throws -> URL {
        let expected: (bytes: Int, sha256: String)
        switch track {
        case 2: expected = (99_654_284, "bee34ef8aef0a58933f90d63766eafc3f8daf447aae64ea35157d84c6850596a")
        case 3: expected = (71_195_084, "80b28a77ea82e454aa6babeb788746d2a141a932f4d101d7c4a8ecc51870ae3d")
        default: throw NativeCDMusicError.unsupportedTrack(track)
        }
        guard let url = url else { throw NativeCDMusicError.missingOriginal(track) }
        guard url.isFileURL, url.query == nil, url.fragment == nil,
              url.host == nil || url.host == "" || url.host == "localhost" else {
            throw NativeCDMusicError.invalidOriginal(track)
        }
        let fileURL = url.standardizedFileURL
        do {
            let info = try fileURL.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
            guard info.isRegularFile == true, info.fileSize == expected.bytes else {
                throw NativeCDMusicError.invalidOriginal(track)
            }
            let file = try FileHandle(forReadingFrom: fileURL)
            defer { try? file.close() }
            var hash = SHA256()
            var bytes = 0
            while let chunk = try file.read(upToCount: 1_048_576), !chunk.isEmpty {
                bytes += chunk.count
                guard bytes <= expected.bytes else { throw NativeCDMusicError.invalidOriginal(track) }
                hash.update(data: chunk)
            }
            let digest = hash.finalize().map { String(format: "%02x", $0) }.joined()
            guard bytes == expected.bytes, digest == expected.sha256 else {
                throw NativeCDMusicError.invalidOriginal(track)
            }
        } catch let error as NativeCDMusicError {
            throw error
        } catch {
            throw NativeCDMusicError.missingOriginal(track)
        }
        return fileURL
    }
}

/// Main-actor confined, including resource destruction and delegate delivery.
/// No player/device is created until the host issues a validated explicit Play.
final class NativeCDMusicAVBackend: NSObject, NativeCDMusicBackend, AVAudioPlayerDelegate {
    private enum DeviceError: Error { case refused }
    private var player: AVAudioPlayer?
    var onCompletion: ((NativeCDMusicCompletion) -> Void)?
    var volume: Float {
        get { player?.volume ?? 0 }
        set { player?.volume = newValue }
    }

    init(url: URL) throws {
        precondition(Thread.isMainThread)
        let audio = try AVAudioPlayer(contentsOf: url)
        self.player = audio
        super.init()
        audio.numberOfLoops = 0
        audio.delegate = self
    }

    init(data: Data) throws {
        precondition(Thread.isMainThread)
        let audio = try AVAudioPlayer(data: data, fileTypeHint: AVFileType.wav.rawValue)
        self.player = audio
        super.init()
        audio.numberOfLoops = 0
        audio.delegate = self
    }

    var deviceEvidence: [String: Any] {
        ["isPlaying": player?.isPlaying ?? false, "currentTime": player?.currentTime ?? 0,
         "duration": player?.duration ?? 0, "channels": player?.numberOfChannels ?? 0,
         "sampleRate": player?.format.sampleRate ?? 0, "loops": player?.numberOfLoops ?? 0]
    }

    func play() throws {
        precondition(Thread.isMainThread)
        guard player?.play() == true else { throw DeviceError.refused }
    }

    func stop() {
        precondition(Thread.isMainThread)
        let old = player
        player = nil
        old?.delegate = nil
        old?.stop()
    }

    isolated deinit {
        player?.delegate = nil
        player?.stop()
    }

    func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        deliver(flag ? .ended : .failed)
    }

    func audioPlayerDecodeErrorDidOccur(_ player: AVAudioPlayer, error: Error?) {
        deliver(.failed)
    }

    private func deliver(_ completion: NativeCDMusicCompletion) {
        DispatchQueue.main.async { [weak self] in self?.onCompletion?(completion) }
    }
}
