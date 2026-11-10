import Foundation

/// Stage-start difficulty preset (RL_DIFICULDADE_*). Normal is the original
/// game; Easy and Hard are presets of this port built from the original's own
/// knobs. The engine applies it inside the stage load, so a change made during
/// a stage takes effect at the next New game, Restart or Next stage.
enum NativeDifficulty: UInt32, CaseIterable, Sendable {
    case easy, normal, hard

    static let preferenceKey = "RisingLandsDifficulty"
    var title: String { ["Fácil", "Normal", "Difícil"][Int(rawValue)] }

    static func load(_ defaults: UserDefaults = .standard) -> NativeDifficulty {
        guard defaults.object(forKey: preferenceKey) != nil,
              let value = UInt32(exactly: defaults.integer(forKey: preferenceKey)) else { return .normal }
        return NativeDifficulty(rawValue: value) ?? .normal
    }
    func store(_ defaults: UserDefaults = .standard) { defaults.set(Int(rawValue), forKey: Self.preferenceKey) }
}

/// Window presentation choices. Host only: never reach Rust, saves or replay.
struct NativeDisplayOptions: Equatable, Sendable {
    static let integerKey = "RisingLandsIntegerScale"
    static let smoothKey = "RisingLandsSmoothFilter"
    var integerScale = false
    var smoothFilter = false

    static func load(_ defaults: UserDefaults = .standard) -> NativeDisplayOptions {
        NativeDisplayOptions(integerScale: defaults.bool(forKey: integerKey), smoothFilter: defaults.bool(forKey: smoothKey))
    }
    func store(_ defaults: UserDefaults = .standard) {
        defaults.set(integerScale, forKey: Self.integerKey)
        defaults.set(smoothFilter, forKey: Self.smoothKey)
    }
}
