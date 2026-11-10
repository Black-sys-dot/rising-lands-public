import Foundation

// All deadlines are monotonic. Initialization is bounded from launch; progress
// never extends a deadline. The original 45-second check budget is unchanged.
struct SmokeReadiness {
    static let windowBudget: TimeInterval = 45
    static let initializationBudget: TimeInterval = 90
    static let checksBudget: TimeInterval = 45
    let launched: TimeInterval
    var windowVisibleAt: TimeInterval?
    var documentLoadedAt: TimeInterval?
    var readyAt: TimeInterval?
    var checksStartedAt: TimeInterval?

    var phase: String {
        if checksStartedAt != nil { return "checks" }
        return windowVisibleAt == nil ? "window" : "initializing"
    }
    mutating func windowVisible(_ now: TimeInterval) { if windowVisibleAt == nil { windowVisibleAt = now } }
    mutating func documentLoaded(_ now: TimeInterval) { documentLoadedAt = now }
    mutating func ready(_ now: TimeInterval) { if readyAt == nil { readyAt = now } }
    func timeout(_ now: TimeInterval) -> String? {
        if let started = checksStartedAt {
            return now - started >= Self.checksBudget ? "checks_timeout" : nil
        }
        if (windowVisibleAt == nil && now - launched >= Self.windowBudget)
            || (windowVisibleAt.map { $0 - launched >= Self.windowBudget } ?? false) { return "window_timeout" }
        return now - launched >= Self.initializationBudget ? "initialization_timeout" : nil
    }
    mutating func startChecks(_ now: TimeInterval) -> Bool {
        guard checksStartedAt == nil, windowVisibleAt != nil, documentLoadedAt != nil,
              readyAt != nil, timeout(now) == nil else { return false }
        checksStartedAt = now
        return true
    }
    func diagnostics(_ now: TimeInterval) -> [String: Any] {
        return ["phase": phase, "elapsedSeconds": now - launched,
                "windowVisibleSignal": windowVisibleAt != nil,
                "documentLoaded": documentLoadedAt != nil, "appReady": readyAt != nil,
                "initializationSeconds": (checksStartedAt ?? now) - launched,
                "checksSeconds": checksStartedAt.map { now - $0 } ?? 0,
                "windowBudgetSeconds": Self.windowBudget,
                "initializationBudgetSeconds": Self.initializationBudget,
                "checksBudgetSeconds": Self.checksBudget]
    }
}
