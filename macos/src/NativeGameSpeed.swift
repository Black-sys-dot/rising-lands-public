import Foundation

/// Player-facing game speed over the engine's virtual TickClock. One logic tick
/// is one original frame and the engine clock advances by exactly one tick per
/// frame at every speed, so speed only changes how many frames the host runs
/// per wall-clock second. No frame is skipped, merged or rendered differently.
enum NativeGameSpeed: Int, CaseIterable, Sendable {
    case half, normal, double, quadruple

    /// The engine's nominal TickClock rate (ticks_per_second at mission start). Host
    /// pacing never changes it; it is the virtual time unit, not the play pace.
    static let tickClockRate = 20.0
    /// Game frames per wall-clock second at 1x. The original's loop is uncapped
    /// (static pacing audit, docs/MACOS_PLAYABLE.md), so this is a host choice: the
    /// user found 20 frames/s too slow in play and chose 30 (2026-11-10).
    static let normalFrameRate = 30.0
    static let preferenceKey = "RisingLandsGameSpeed"

    var multiplier: Double { [0.5, 1, 2, 4][rawValue] }
    /// Game frames per wall-clock second at this speed.
    var frameRate: Double { Self.normalFrameRate * multiplier }
    var label: String { ["0,5x", "1x", "2x", "4x"][rawValue] }
    var menuTitle: String { "\(label) · \(Int(frameRate)) quadros/s" }

    init?(frameRate: Double) {
        guard let speed = Self.allCases.first(where: { $0.frameRate == frameRate }) else { return nil }
        self = speed
    }
    var faster: NativeGameSpeed { NativeGameSpeed(rawValue: min(rawValue + 1, 3))! }
    var slower: NativeGameSpeed { NativeGameSpeed(rawValue: max(rawValue - 1, 0))! }

    static func load(_ defaults: UserDefaults = .standard) -> NativeGameSpeed {
        guard defaults.object(forKey: preferenceKey) != nil,
              let speed = NativeGameSpeed(rawValue: defaults.integer(forKey: preferenceKey)) else { return .normal }
        return speed
    }
    func store(_ defaults: UserDefaults = .standard) { defaults.set(rawValue, forKey: Self.preferenceKey) }
}

/// Host frame scheduling only: decides when the worker runs the next complete
/// frame. A rate of 0 runs frames back to back (diagnostic "sem limite").
/// Late frames still run one by one; wall-clock debt beyond `maximumBacklog`
/// (a stall, a sheet, a hidden window) is forgiven instead of replayed as a
/// burst. Pause/resume and speed changes re-anchor the deadline.
struct NativeFramePacer: Sendable {
    static let maximumBacklog = 0.25
    private(set) var rate: Double
    private(set) var deadline: Double

    init(rate: Double, now: Double) {
        self.rate = max(0, rate)
        deadline = now
    }

    mutating func restart(now: Double) { deadline = now }

    mutating func setRate(_ rate: Double, now: Double) {
        self.rate = max(0, rate)
        deadline = now
    }

    /// Returns 0 and books one frame when it is due; otherwise the seconds to wait.
    mutating func delay(now: Double) -> Double {
        guard rate > 0 else { return 0 }
        if now < deadline { return deadline - now }
        if now - deadline > Self.maximumBacklog { deadline = now }
        deadline += 1 / rate
        return 0
    }
}
