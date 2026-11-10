import Foundation

/// Presentation lifetime only. Consume the original requested track; the host
/// plays the first request while idle, without changing the no-CD simulation.
/// This is a modern playback policy, not an initialized-CD engine schedule.
@MainActor final class NativeCDMusicHost {
    let player: NativeCDMusicPlayer
    private(set) var stageActive = false
    private(set) var notice = "Música do CD · aguardando pedido do jogo."
    private var rejectedTracks=Set<Int>()
    private var automatic=true
    private var transportError:Int32?
    private(set) var requests=0, suppressed=0
    private(set) var starts = 0
    private(set) var finishes = 0
    private(set) var failures = 0
    var onChange: (() -> Void)?

    init(player: NativeCDMusicPlayer) {
        self.player = player
        player.onEvent = { [weak self] event in
            guard let self = self else { return }
            switch event {
            case .started(let track): self.starts += 1; self.notice = "Música do CD · faixa \(track)"
            case .finished: self.finishes += 1; self.notice = "Música do CD encerrada."
            case .failed(let error):
                switch error {
                case .playbackFailed(let track), .missingOriginal(let track), .invalidOriginal(let track): self.rejectedTracks.insert(track)
                default: break
                }
                self.failures += 1; self.notice = error.localizedDescription
            case .stopped: self.notice = "Música do CD parada."
            case .closed: self.notice = "Música do CD encerrada."
            }
            self.onChange?()
        }
    }

    func beginStage() { guard player.state != .closed else { return }; stop(); rejectedTracks.removeAll();transportError=nil;automatic=true;stageActive = true }
    // Result 0x13 bypasses generic MenuInit Stop: inherit the current stream,
    // without selecting a result track or advancing any playlist after EOF.
    func result() { stageActive = false }
    func leaveStage() { stageActive = false; stop() }
    func stop() { do { try player.execute(.stop) } catch { report(error) } }
    func close() { stageActive = false; do { try player.execute(.close) } catch { report(error) } }
    func audition(track: Int) {
        guard stageActive else { notice = "Inicie uma fase no modo Jogar para ouvir o CD."; onChange?(); return }
        do { try player.execute(.play(track: track)) } catch { report(error) }
    }
    /// Never infer a track from a stage, guard byte or local random source.
    func request(site:UInt32,track:Int,loop:Int) {
        guard stageActive,automatic else{return}
        requests += 1
        guard site==0x42d32d || site==0x4684bd,loop==0 else {
            transportFailed(-1);return
        }
        if site==0x42d32d,case .playing = player.state { suppressed += 1;return }
        guard !rejectedTracks.contains(track) else{return}
        do { try player.execute(.play(track:track)) }
        catch { rejectedTracks.insert(track);report(error) }
    }
    func transportFailed(_ code:Int32) {
        guard transportError != code else{return};transportError=code;automatic=false;failures += 1
        notice="Pedidos de música indisponíveis (\(code)).";onChange?()
    }
    func stopAutomatic(){automatic=false;stop()}
    func mute(_ value: Bool) { do { try player.execute(.setMuted(value)) } catch { report(error) } }
    private func report(_ error: Error) { failures += 1; notice = error.localizedDescription; onChange?() }
    var metrics: [String: Any] {
        let track: Int
        if case .playing(let value) = player.state { track = value } else { track = 0 }
        return ["automaticSelectionWired": true, "selectionPolicy": "first original request while idle; no-CD simulation unchanged",
                "exactInitializedCDSchedule":false,"requests":requests,"suppressedWhilePlaying":suppressed,"rejectedTracks":rejectedTracks.sorted(),
                "track": track, "stageActive": stageActive, "starts": starts, "finishes": finishes,
                "failures": failures, "muted": player.isMuted, "notice": notice, "device": player.deviceEvidence]
    }
}
