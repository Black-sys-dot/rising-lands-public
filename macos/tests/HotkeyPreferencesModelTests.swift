import Foundation

/// Headless: no NSApplication, controller, NSView, window or user preferences.
@main
@MainActor
struct HotkeyPreferencesModelTests {
    typealias Bindings = NativeHotkeyBindings
    typealias Session = NativeHotkeyPreferencesSession
    struct Failure: Error { let message: String }
    enum Rejected: Error { case host }
    final class PreparationControl { var reject = false }

    static func check(_ condition: @autoclosure () throws -> Bool, _ message: String,
                      line: UInt = #line) throws {
        if try !condition() { throw Failure(message: "Line \(line): \(message)") }
    }

    static func rejects(_ operation: () throws -> Void) throws {
        do { try operation() } catch { return }
        throw Failure(message: "Expected failure")
    }

    static func main() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("HotkeyPreferences-\(UUID())")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("bindings.json")
        let original = Bindings()
        try original.save(to: url)
        let originalBytes = try Data(contentsOf: url)
        var live = original
        var prepares = 0
        var commits = 0
        let preparation = PreparationControl()
        var staged: Bindings?
        let session = Session(bindings: live, configurationURL: url) { candidate in
            prepares += 1
            if preparation.reject { throw Rejected.host }
            staged = candidate
            // An install sees the successfully saved file, never a stale document.
            return {
                var saved = Bindings()
                do { try saved.load(from: url) }
                catch { preconditionFailure("Commit ran without saved configuration") }
                precondition(saved == candidate)
                live = candidate
                commits += 1
            }
        }
        try check(!session.hasChanges && session.capture == nil, "Nonempty initial draft")
        try session.apply()
        try check(prepares == 0 && commits == 0, "No-op apply called host")
        try check(Data(contentsOf: url) == originalBytes, "Construction/no-op wrote preferences")
        print("PASS initialization and no-op apply have no host/file effects")

        try session.beginCapture(.enter, replacing: 0)
        try check(try !session.record(keyCode: 15, modifiers: .control, isRepeat: true), "Repeat recorded")
        try check(session.draft == original && session.capture != nil, "Repeat changed draft/capture")
        try rejects { try session.record(keyCode: 123, modifiers: .shift) }
        try check(session.draft == original && session.capture != nil, "Conflict partially applied")
        try rejects { try session.record(keyCode: 15, modifiers: .command) }
        try rejects { try session.record(keyCode: 55, modifiers: []) }
        try rejects { try session.apply() }
        try check(commits == 0 && prepares == 0, "Pending capture applied")
        try session.record(keyCode: 15, modifiers: .control)
        try check(session.capture == nil && session.hasChanges, "Valid capture not finished")
        try check(session.draft.chords(for: .enter).map(\.keyCode) == [15, 76], "Lost untouched Enter alias")
        try check(session.draft.action(keyCode: 15, modifiers: .control) == .enter, "Wrong captured channel")
        try check(session.draft.action(keyCode: 15, modifiers: [.control, .shift]) == nil, "Captured chord not exact")
        try check(live == original && Data(contentsOf: url) == originalBytes, "Capture escaped draft")
        print("PASS capture retries, repeat rejection, Command reservation, conflicts and alias retention")

        try session.beginCapture(.enter, replacing: nil)
        try rejects { try session.record(keyCode: 76, modifiers: []) }
        session.cancelCapture()
        try session.beginCapture(.enter, replacing: nil)
        try session.record(keyCode: 17, modifiers: .option)
        try check(session.draft.chords(for: .enter).map(\.keyCode) == [15, 76, 17], "Alternative not appended")
        try session.removeAlias(.enter, at: 1)
        try check(session.draft.chords(for: .enter).map(\.keyCode) == [15, 17], "Wrong alias removed")
        let beforeInvalid = session.draft
        try rejects { try session.beginCapture(.enter, replacing: 9) }
        try rejects { try session.removeAlias(.enter, at: -1) }
        try check(session.draft == beforeInvalid, "Invalid selection changed draft")
        session.cancel()
        try check(session.draft == original && !session.hasChanges && session.capture == nil, "Cancel retained edits")
        try check(Data(contentsOf: url) == originalBytes && live == original, "Cancel changed file/live")
        print("PASS add/remove alternatives, duplicate rejection, invalid selection and cancel isolation")

        try session.beginCapture(.x, replacing: 0)
        try session.record(keyCode: 15, modifiers: .control)
        let candidate = session.draft
        preparation.reject = true
        try rejects { try session.apply() }
        try check(live == original && session.applied == original, "Failed prepare changed live/baseline")
        try check(Data(contentsOf: url) == originalBytes, "Failed prepare wrote file")
        try check(session.draft == candidate && !session.isApplying, "Failed prepare discarded edits/locked session")
        preparation.reject = false
        try session.apply()
        try check(live == candidate && session.applied == candidate && !session.hasChanges, "Install incomplete")
        try check(prepares == 2 && commits == 1 && staged == candidate, "Prepare/install count/order")
        let committedBytes = try Data(contentsOf: url)
        print("PASS failed host prepare preserves bytes/live; success saves before one installation")

        let directoryTarget = directory.appendingPathComponent("directory-target")
        try FileManager.default.createDirectory(at: directoryTarget, withIntermediateDirectories: false)
        let sentinel = directoryTarget.appendingPathComponent("keep")
        try originalBytes.write(to: sentinel)
        var badCommits = 0
        var badPrepares = 0
        let saveFailure = Session(bindings: live, configurationURL: directoryTarget) { _ in
            badPrepares += 1
            return { badCommits += 1 }
        }
        try saveFailure.beginCapture(.escape, replacing: nil)
        try saveFailure.record(keyCode: 17, modifiers: .option)
        try rejects { try saveFailure.apply() }
        try check(badPrepares == 1 && badCommits == 0, "Save failure installed")
        try check(saveFailure.applied == live && saveFailure.hasChanges && !saveFailure.isApplying,
                  "Save failure changed applied baseline or discarded draft")
        try check(Data(contentsOf: sentinel) == originalBytes && Data(contentsOf: url) == committedBytes,
                  "Save failure damaged previous configuration")
        print("PASS failed atomic save preserves live/configuration and retains retryable draft")

        try session.resetDefaults()
        try check(session.draft == original && live == candidate, "Reset escaped draft")
        session.cancel()
        try check(session.draft == candidate, "Cancel reset lost applied customization")
        try session.resetDefaults()
        try session.apply()
        try check(live == original && session.applied == original, "Applied reset not installed")
        try check(live.chords(for: .enter).map(\.keyCode) == [36, 76], "Reset lost default aliases")
        try check(Data(contentsOf: url) == originalBytes, "Reset changed file format/defaults")
        try session.removeAlias(.escape, at: 0)
        try session.apply()
        var loaded = Bindings()
        try loaded.load(from: url)
        try check(loaded.chords(for: .escape).isEmpty, "Unbound action not persisted")
        try check(commits == 3, "Wrong successful commit count")
        print("PASS explicit reset/apply/cancel and unbound action persistence in existing format")

        let badURL = Session(bindings: original, configurationURL: URL(string: "https://example.invalid")!) { _ in
            return { preconditionFailure("Remote URL installed") }
        }
        try badURL.removeAlias(.x, at: 0)
        try rejects { try badURL.apply() }
        try check(badURL.applied == original && badURL.hasChanges, "Non-file URL changed baseline")
        let safeMessages: [Error] = [Bindings.BindingError.invalidKeyCode(65535),
                                    Bindings.BindingError.writeFailed, Rejected.host]
        for error in safeMessages {
            let text = Session.message(for: error)
            try check(!text.isEmpty && !text.contains("65535") && !text.contains("host"), "UI leaks implementation")
        }
        print("PASS non-file URL rejection and player-facing error messages")
        print("HotkeyPreferencesModelTests: 7 groups passed; no AppKit UI instantiated")
    }
}
