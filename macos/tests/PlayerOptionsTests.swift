import Foundation

// Standalone headless executable. Compile ONLY with NativePlayerOptions.swift.
@main
struct PlayerOptionsTests {
    static func expect(_ condition: Bool, _ label: String) {
        if !condition { FileHandle.standardError.write(Data("PlayerOptionsTests FAILED: \(label)\n".utf8)); exit(1) }
    }
    static func main() {
        let suite = "PlayerOptionsTests-\(ProcessInfo.processInfo.processIdentifier)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        expect(NativeDifficulty.allCases.map(\.rawValue) == [0, 1, 2], "values are RL_DIFICULDADE_*")
        expect(NativeDifficulty.allCases.map(\.title) == ["Fácil", "Normal", "Difícil"], "PT-BR titles")
        expect(NativeDifficulty.load(defaults) == .normal, "Normal, the original game, by default")
        NativeDifficulty.hard.store(defaults)
        expect(NativeDifficulty.load(defaults) == .hard, "stored preset survives")
        defaults.set(9, forKey: NativeDifficulty.preferenceKey)
        expect(NativeDifficulty.load(defaults) == .normal, "unknown value falls back to Normal")
        defaults.set(-1, forKey: NativeDifficulty.preferenceKey)
        expect(NativeDifficulty.load(defaults) == .normal, "negative value falls back to Normal")
        expect(NativeDisplayOptions.load(defaults) == NativeDisplayOptions(), "fit and sharp pixels by default")
        let chosen = NativeDisplayOptions(integerScale: true, smoothFilter: true)
        chosen.store(defaults)
        expect(NativeDisplayOptions.load(defaults) == chosen, "display options survive")
        print("PlayerOptionsTests passed")
    }
}
