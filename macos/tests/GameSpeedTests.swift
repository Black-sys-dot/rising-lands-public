import Foundation

// Standalone headless executable. Compile ONLY with NativeGameSpeed.swift.
@main
struct GameSpeedTests {
    static var checks = 0
    static func expect(_ condition: Bool, _ label: String) {
        checks += 1
        if !condition { FileHandle.standardError.write(Data("GameSpeedTests FAILED: \(label)\n".utf8)); exit(1) }
    }

    /// Simulated worker: runs a frame whenever the pacer says so, otherwise
    /// sleeps exactly the returned delay (or until a scheduled event). Each frame
    /// costs `work` seconds of wall time.
    static func run(_ pacer: inout NativeFramePacer, from start: Double, to end: Double, work: Double = 0.001,
                    stall: (at: Double, seconds: Double)? = nil) -> [Double] {
        var now = start, frames = [Double](), stalled = false
        while now < end {
            if let stall, !stalled, now >= stall.at { now += stall.seconds; stalled = true; continue }
            let wait = pacer.delay(now: now)
            if wait > 0 { now += wait; continue }
            frames.append(now); now += work
        }
        return frames
    }

    static func main() {
        expect(NativeGameSpeed.allCases.map(\.frameRate) == [15, 30, 60, 120], "speeds over the 30 frames/s 1x pace")
        expect(NativeGameSpeed.allCases.map(\.label) == ["0,5x", "1x", "2x", "4x"], "PT-BR labels")
        expect(NativeGameSpeed(frameRate: 60) == .double && NativeGameSpeed(frameRate: 40) == nil, "rate lookup")
        expect(NativeGameSpeed.quadruple.faster == .quadruple && NativeGameSpeed.half.slower == .half &&
               NativeGameSpeed.normal.faster == .double && NativeGameSpeed.normal.slower == .half, "bounded steps")
        let suite = "GameSpeedTests.\(ProcessInfo.processInfo.processIdentifier)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        expect(NativeGameSpeed.load(defaults) == .normal, "default speed is 1x")
        NativeGameSpeed.quadruple.store(defaults)
        expect(NativeGameSpeed.load(defaults) == .quadruple, "speed preference round trip")
        defaults.set(9, forKey: NativeGameSpeed.preferenceKey)
        expect(NativeGameSpeed.load(defaults) == .normal, "invalid stored speed falls back to 1x")

        // Each speed runs rate×seconds frames (±1) and never two frames closer
        // than its period (no burst), with any per-frame cost below the period.
        for speed in NativeGameSpeed.allCases {
            var pacer = NativeFramePacer(rate: speed.frameRate, now: 100)
            let frames = run(&pacer, from: 100, to: 130, work: 0.002)
            expect(abs(Double(frames.count) - speed.frameRate * 30) <= 1, "\(speed.label) frame count \(frames.count)")
            let gaps = zip(frames.dropFirst(), frames).map { $0 - $1 }
            expect(gaps.allSatisfy { $0 >= 1 / speed.frameRate - 1e-9 }, "\(speed.label) no burst")
        }
        // Slow frames (longer than the period) run back to back without skipping
        // and fall behind smoothly rather than accumulating unbounded debt.
        var slow = NativeFramePacer(rate: 80, now: 0)
        let slowFrames = run(&slow, from: 0, to: 10, work: 0.02)
        expect(abs(slowFrames.count - 500) <= 1, "throughput-bound 4x runs every frame back to back (\(slowFrames.count))")
        // A 3-second stall is forgiven: the next second runs at the normal rate,
        // not a catch-up burst of 60 extra frames.
        var stalled = NativeFramePacer(rate: 20, now: 0)
        let afterStall = run(&stalled, from: 0, to: 6, stall: (at: 2, seconds: 3))
        let burst = afterStall.filter { $0 >= 5 && $0 < 6 }.count
        expect(burst <= 21 + Int(NativeFramePacer.maximumBacklog * 20), "stall forgiven, no burst (\(burst))")
        expect(afterStall.filter { $0 < 2 }.count == 40, "frames before the stall")
        // Pause is simply not calling the pacer; resume re-anchors immediately.
        var paused = NativeFramePacer(rate: 20, now: 0)
        _ = run(&paused, from: 0, to: 1)
        paused.restart(now: 50)
        expect(paused.delay(now: 50) == 0 && paused.delay(now: 50) > 0.049, "resume runs one frame then waits a period")
        // A speed change re-anchors at the new period.
        var changing = NativeFramePacer(rate: 10, now: 0)
        _ = changing.delay(now: 0)
        changing.setRate(80, now: 0.01)
        expect(changing.delay(now: 0.01) == 0 && abs(changing.delay(now: 0.01) - 1.0 / 80) < 1e-9, "speed change re-anchors")
        // Diagnostic unlimited rate never waits.
        var unlimited = NativeFramePacer(rate: 0, now: 0)
        expect((0..<100).allSatisfy { _ in unlimited.delay(now: 0) == 0 }, "unlimited never waits")
        print("GameSpeedTests: \(checks) checks passed; 4 speeds over the 30 frames/s 1x pace, no burst, bounded backlog, pause/resume and speed re-anchor")
    }
}
