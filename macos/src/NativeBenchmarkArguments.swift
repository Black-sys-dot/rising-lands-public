import Foundation

/// Stage selection for the explicit `--native-benchmark` diagnostic only.
/// Normal play never reads these flags.
struct NativeBenchmarkArguments: Equatable {
    struct Invalid: Error, LocalizedError, Equatable {
        let reason: String
        var errorDescription: String? { "InvalidNativeBenchmarkArguments: \(reason)" }
    }
    let mission: String
    let difficulty: UInt32

    /// `--benchmark-mission MONDExx[A-D].ETA` (default MONDE01.ETA) and
    /// `--benchmark-difficulty 0|1|2` (default 1, Normal). Test mode keeps its
    /// historical MONDE01/Normal proof; the order gestures target MONDE01 pixels.
    static func parse(_ args: [String], mode: String) throws -> NativeBenchmarkArguments {
        func value(_ key: String) -> String? {
            guard let index = args.firstIndex(of: key) else { return nil }
            return args.count > index + 1 ? args[index + 1] : ""
        }
        let mission = (value("--benchmark-mission") ?? "MONDE01.ETA").uppercased()
        guard mission.range(of: "^MONDE(0[1-9]|1[0-9]|2[0-5])[A-D]?\\.ETA$", options: .regularExpression) != nil else {
            throw Invalid(reason: "mission \(mission)")
        }
        guard let difficulty = UInt32(value("--benchmark-difficulty") ?? "1"), difficulty <= 2 else {
            throw Invalid(reason: "difficulty")
        }
        let monde01 = mission == "MONDE01.ETA" && difficulty == 1
        if mode == "test" && !monde01 { throw Invalid(reason: "test mode is the MONDE01 Normal proof") }
        if args.contains("--benchmark-orders") && mission != "MONDE01.ETA" {
            throw Invalid(reason: "order gestures are MONDE01 coordinates")
        }
        return NativeBenchmarkArguments(mission: mission, difficulty: difficulty)
    }
}
