import Foundation

@main
@MainActor
struct HotkeyInputRouterTests {
    typealias Router = NativeHotkeyInputRouter
    typealias Bindings = NativeHotkeyBindings
    typealias Event = Router.KeyEvent
    typealias Modifiers = Bindings.Modifiers
    struct Failure: Error { let message: String }
    enum Rejected: Error { case host }

    @MainActor
    final class Capture {
        var events: [Event] = []
        var trace: [String] = []
        func receive(_ event: Event) {
            events.append(event)
            trace.append("\(event.virtualKey):\(event.down ? "down" : "up")")
        }
    }

    static func check(_ value: Bool, _ message: String) throws {
        if !value { throw Failure(message: message) }
    }
    static func event(_ key: Int, _ down: Bool) -> Event { Event(virtualKey: key, down: down) }
    static func make(_ bindings: Bindings = Bindings()) -> (Router, Capture) {
        let capture = Capture()
        let router = Router(bindings: bindings, sink: { capture.receive($0) })
        router.setActive(true)
        return (router, capture)
    }
    static func rejects(_ body: () throws -> Void) throws {
        do { try body() } catch is Failure { throw Failure(message: "Unexpected assertion failure") }
        catch { return }
        throw Failure(message: "Expected rejection")
    }

    static func baseline() throws {
        // Independent table from the committed host, not from router internals.
        let table: [(UInt16, Int)] = [(36,13),(76,13),(53,27),(49,32),(123,37),
            (126,38),(124,39),(125,40),(117,46),(7,88),(48,9),(29,48),(18,49),(19,50),
            (20,51),(21,52),(23,53),(22,54),(26,55),(28,56),(25,57)]
        for (code, key) in table {
            for bits in UInt8(0)...7 {
                let (router, capture) = make()
                let modifiers = Modifiers(rawValue: bits)
                try check(router.keyDown(keyCode: code, modifiers: modifiers) == .handled, "Default missed")
                router.keyUp(keyCode: code, modifiers: [])
                var expected: [Event] = []
                for (mask, vkey): (UInt8, Int) in [(1,16),(2,17),(4,18)] where bits & mask != 0 {
                    expected.append(event(vkey, true))
                }
                expected.append(event(key, true))
                for (mask, vkey): (UInt8, Int) in [(1,16),(2,17),(4,18)] where bits & mask != 0 {
                    expected.append(event(vkey, false))
                }
                expected.append(event(key, false))
                try check(capture.events == expected && router.heldKeys.isEmpty,
                          "Committed default/modifier order differs at \(code)/\(bits)")
            }
        }
    }

    static func modifierRelease() throws {
        var bindings = Bindings()
        try bindings.rebind(.x, to: [.init(keyCode: 15, modifiers: [.shift, .control, .option])])
        let (router, capture) = make(bindings)
        router.keyDown(keyCode: 15, modifiers: [.shift, .control, .option])
        router.synchronizeModifiers([]) // flagsChanged arrives before keyUp
        router.keyUp(keyCode: 15, modifiers: [])
        try check(capture.events == [event(16,true),event(17,true),event(18,true),event(88,true),
            event(16,false),event(17,false),event(18,false),event(88,false)], "Lost latched chord on modifier release")
        try check(router.heldKeys.isEmpty, "Chord stuck")
    }

    static func aliases() throws {
        for order: [UInt16] in [[36,76],[76,36]] {
            let (router, capture) = make()
            router.keyDown(keyCode: 36, modifiers: [])
            router.keyDown(keyCode: 76, modifiers: [])
            router.keyDown(keyCode: 76, modifiers: [], isRepeat: true)
            router.keyUp(keyCode: order[0], modifiers: [])
            router.keyUp(keyCode: order[0], modifiers: [])
            try check(capture.events == [event(13,true)] && router.heldKeys == [13], "Alias released early")
            router.keyUp(keyCode: order[1], modifiers: [])
            try check(capture.events == [event(13,true),event(13,false)] && router.heldKeys.isEmpty,
                      "Last alias did not release exactly once")
        }
    }

    static func repeatsAndOrphans() throws {
        let (router, capture) = make()
        router.keyUp(keyCode: 123, modifiers: [])
        router.keyDown(keyCode: 123, modifiers: .shift, isRepeat: true)
        try check(capture.events.isEmpty, "Orphan repeat/up injected gameplay")
        router.keyDown(keyCode: 123, modifiers: [])
        router.keyDown(keyCode: 123, modifiers: .option) // duplicate down without repeat bit
        router.keyDown(keyCode: 123, modifiers: .control, isRepeat: true)
        router.keyUp(keyCode: 123, modifiers: [])
        router.keyUp(keyCode: 123, modifiers: [])
        try check(capture.events == [event(37,true),event(37,false)], "Duplicate movement packet")
    }

    static func exactPhysicalLatch() throws {
        var bindings = Bindings()
        try bindings.rebind(.x, to: [.init(keyCode: 15, modifiers: .control)])
        try bindings.rebind(.escape, to: [.init(keyCode: 15, modifiers: .option)])
        let (router, capture) = make(bindings)
        router.keyDown(keyCode: 15, modifiers: []) // unmatched physical down is remembered
        router.keyDown(keyCode: 15, modifiers: .control)
        router.keyUp(keyCode: 15, modifiers: [])
        try check(capture.events.isEmpty, "Unmatched down reinterpreted after modifiers changed")
        router.keyDown(keyCode: 15, modifiers: .control)
        router.keyUp(keyCode: 15, modifiers: .option)
        router.synchronizeModifiers([])
        try check(capture.events == [event(17,true),event(88,true),event(17,false),event(18,true),
                                    event(88,false),event(18,false)], "Up resolved new chord instead of latch")
    }

    static func focus() throws {
        let capture = Capture()
        let router = Router(sink: { capture.receive($0) })
        router.keyDown(keyCode: 7, modifiers: .shift)
        try check(capture.events.isEmpty && !router.isActive, "Input initially active")
        router.setActive(true)
        router.keyDown(keyCode: 7, modifiers: [.shift,.control,.option])
        router.keyDown(keyCode: 76, modifiers: [.shift,.control,.option])
        router.keyDown(keyCode: 36, modifiers: [.shift,.control,.option])
        router.keyDown(keyCode: 123, modifiers: [.shift,.control,.option])
        let downs = capture.events
        router.setActive(false)
        let expected = downs + [13,16,17,18,37,88].map { event($0,false) }
        try check(capture.events == expected && router.heldKeys.isEmpty, "Focus release order")
        router.setActive(false)
        router.keyDown(keyCode: 7, modifiers: .shift)
        router.keyUp(keyCode: 36, modifiers: .shift)
        router.synchronizeModifiers(.shift)
        router.setActive(true)
        router.keyDown(keyCode: 7, modifiers: [], isRepeat: true)
        router.keyUp(keyCode: 7, modifiers: [])
        router.keyUp(keyCode: 76, modifiers: [])
        try check(capture.events == expected, "Focus restored/inactive input duplicated a packet")
        router.keyDown(keyCode: 36, modifiers: [])
        router.keyUp(keyCode: 36, modifiers: [])
        try check(capture.events == expected + [event(13,true),event(13,false)], "Focus left stale alias count")
    }

    static func command() throws {
        let (router, capture) = make()
        for code: UInt16 in [36,76,53,49,123,126,124,125,117,7,29,18,19,20,21,23,22,26,28,25,15] {
            try check(router.keyDown(keyCode: code, modifiers: [.command,.shift]) == .nativeMenu,
                      "Command down consumed")
            try check(router.keyUp(keyCode: code, modifiers: .command) == .nativeMenu, "Command up consumed")
        }
        try check(capture.events.isEmpty, "Menu chord injected gameplay")
        router.keyDown(keyCode: 7, modifiers: [])
        router.keyDown(keyCode: 7, modifiers: .command, isRepeat: true)
        try check(router.keyUp(keyCode: 7, modifiers: [.command,.control]) == .nativeMenu, "Command up stolen")
        router.synchronizeModifiers([])
        try check(capture.events == [event(88,true),event(17,true),event(88,false),event(17,false)],
                  "Command prevented old latch release or reordered independent modifier")
    }

    static func invalidConfiguration(_ directory: URL) throws {
        let (router, capture) = make()
        router.keyDown(keyCode: 123, modifiers: .shift)
        let original = router.bindings
        let before = capture.events
        for chord: Bindings.Chord in [.init(keyCode: 123), .init(keyCode: 65535),
            .init(keyCode: 56), .init(keyCode: 15, modifiers: .command),
            .init(keyCode: 15, modifiers: .init(rawValue: 16))] {
            var candidate = original
            try rejects { try candidate.rebind(.x, to: [chord]) }
            try check(candidate == original && router.bindings == original && router.heldKeys == [16,37],
                      "Invalid edit changed live configuration/held keys")
        }
        let bad = directory.appendingPathComponent("invalid.json")
        let bytes = Data("{\"version\":1,\"bindings\":[]}".utf8)
        try bytes.write(to: bad)
        var candidate = original
        try rejects { try candidate.load(from: bad) }
        try check(try Data(contentsOf: bad) == bytes, "Bad load rewrote config")
        try check(capture.events == before && router.bindings == original, "Rejected config injected events")
        router.keyUp(keyCode: 123, modifiers: [])
        try check(capture.events == before + [event(16,false),event(37,false)], "Rejected edit lost latch")
    }

    static func transactionFailures(_ directory: URL) throws {
        let (router, capture) = make()
        let url = directory.appendingPathComponent("transaction.json")
        try router.bindings.save(to: url)
        let originalBytes = try Data(contentsOf: url)
        let original = router.bindings
        router.keyDown(keyCode: 36, modifiers: [.shift,.option])
        router.keyDown(keyCode: 76, modifiers: [.shift,.option])
        let before = capture.events
        var candidate = original
        try candidate.rebind(.enter, to: [.init(keyCode: 15)])
        try rejects {
            _ = try router.prepareInstallation(candidate) { _ in throw Rejected.host }
        }
        try check(try Data(contentsOf: url) == originalBytes, "Prepare changed file")
        try check(router.bindings == original && router.heldKeys == [13,16,18] && capture.events == before,
                  "Failed prepare changed held state")
        let blocker = directory.appendingPathComponent("blocker")
        try originalBytes.write(to: blocker)
        try rejects {
            let commit = try router.prepareInstallation(candidate)
            try candidate.save(to: blocker.appendingPathComponent("config.json"))
            commit()
        }
        try check(try Data(contentsOf: url) == originalBytes && Data(contentsOf: blocker) == originalBytes,
                  "Failed save damaged original bytes")
        try check(router.bindings == original && router.heldKeys == [13,16,18] && capture.events == before,
                  "Save failure released live keys")
        router.keyUp(keyCode: 36, modifiers: [.shift,.option])
        try check(router.heldKeys == [13,16,18], "Failed transaction lost alias references")
        router.keyUp(keyCode: 76, modifiers: [])
        try check(capture.events == before + [event(16,false),event(18,false),event(13,false)],
                  "Failure lost modifier-before-final-alias release order")
    }

    static func successfulTransaction(_ directory: URL) throws {
        let (router, capture) = make()
        let url = directory.appendingPathComponent("success.json")
        router.keyDown(keyCode: 123, modifiers: [.shift,.control,.option])
        router.keyDown(keyCode: 36, modifiers: [.shift,.control,.option])
        router.keyDown(keyCode: 76, modifiers: [.shift,.control,.option])
        var candidate = router.bindings
        try candidate.rebind(.left, to: [.init(keyCode: 15)])
        capture.trace = []
        let commit = try router.prepareInstallation(candidate) { proposed in
            try check(proposed == candidate && router.heldKeys == [13,16,17,18,37], "Prepare mutated state")
            capture.trace.append("prepare")
        }
        try check(router.bindings == Bindings(), "Preparation installed early")
        try candidate.save(to: url)
        capture.trace.append("save")
        commit()
        capture.trace.append("commit")
        try check(capture.trace == ["prepare","save","13:up","16:up","17:up","18:up","37:up","commit"],
                  "Transaction callback/injection order")
        var loaded = Bindings()
        try loaded.load(from: url)
        try check(router.bindings == candidate && loaded == candidate && router.heldKeys.isEmpty,
                  "Successful transaction did not install persisted candidate")
        let before = capture.events
        router.keyDown(keyCode: 123, modifiers: [], isRepeat: true)
        router.keyUp(keyCode: 123, modifiers: [])
        router.keyUp(keyCode: 36, modifiers: [])
        router.keyUp(keyCode: 76, modifiers: [])
        try check(capture.events == before, "Old keyUp/repeat injected after commit")
        router.keyDown(keyCode: 15, modifiers: [])
        commit() // one-shot: must not release a later new movement down
        try check(router.heldKeys == [37], "Duplicate commit released new state")
        router.keyUp(keyCode: 15, modifiers: [])
        try check(capture.events == before + [event(37,true),event(37,false)], "New mapping order")
    }

    static func resetAndNoOp(_ directory: URL) throws {
        var custom = Bindings()
        try custom.rebind(.x, to: [.init(keyCode: 15)])
        let (router, capture) = make(custom)
        router.keyDown(keyCode: 15, modifiers: [])
        let same = try router.prepareInstallation(custom)
        same()
        try check(capture.events == [event(88,true)] && router.heldKeys == [88], "No-op installation flushed input")
        var defaults = custom
        defaults.resetToDefaults()
        let commit = try router.prepareInstallation(defaults)
        try defaults.save(to: directory.appendingPathComponent("reset.json"))
        commit()
        router.keyUp(keyCode: 15, modifiers: [])
        router.releaseAllHeldKeys()
        commit()
        try check(capture.events == [event(88,true),event(88,false)], "Reset emitted duplicate order packets")
        router.keyDown(keyCode: 7, modifiers: [])
        router.keyUp(keyCode: 7, modifiers: [])
        try check(capture.events == [event(88,true),event(88,false),event(88,true),event(88,false)], "Reset not installed")
    }

    static func replayPayload() throws {
        func record() throws -> Data {
            let (router, capture) = make()
            router.synchronizeModifiers([.shift,.control,.option]) // before unchanged mouse callback
            router.keyDown(keyCode: 36, modifiers: [.shift,.control,.option])
            router.keyDown(keyCode: 76, modifiers: [.shift,.control,.option])
            router.keyDown(keyCode: 36, modifiers: [.shift,.control,.option], isRepeat: true)
            router.keyUp(keyCode: 36, modifiers: [.shift,.control,.option])
            router.setActive(false)
            // This is the host's existing next-frame envelope, not a new router clock.
            let records = capture.events.map { ["frame": 129, "event": $0.payload] as [String: Any] }
            return try JSONSerialization.data(withJSONObject: records, options: [.sortedKeys])
        }
        let expected = "[{\"event\":{\"down\":true,\"kind\":\"key\",\"vkey\":16},\"frame\":129}," +
            "{\"event\":{\"down\":true,\"kind\":\"key\",\"vkey\":17},\"frame\":129}," +
            "{\"event\":{\"down\":true,\"kind\":\"key\",\"vkey\":18},\"frame\":129}," +
            "{\"event\":{\"down\":true,\"kind\":\"key\",\"vkey\":13},\"frame\":129}," +
            "{\"event\":{\"down\":false,\"kind\":\"key\",\"vkey\":13},\"frame\":129}," +
            "{\"event\":{\"down\":false,\"kind\":\"key\",\"vkey\":16},\"frame\":129}," +
            "{\"event\":{\"down\":false,\"kind\":\"key\",\"vkey\":17},\"frame\":129}," +
            "{\"event\":{\"down\":false,\"kind\":\"key\",\"vkey\":18},\"frame\":129}]"
        let first = try record()
        try check(first == Data(expected.utf8) && first == record(), "Replay payload/order is nondeterministic")
    }

    #if HOTKEY_EDITOR_TESTS
    static func editorTransaction(_ directory: URL) throws {
        let (router, capture) = make()
        let url = directory.appendingPathComponent("editor.json")
        try router.bindings.save(to: url)
        let originalBytes = try Data(contentsOf: url)
        var failPrepare = true
        let session = NativeHotkeyPreferencesSession(bindings: router.bindings, configurationURL: url) { candidate in
            try router.prepareInstallation(candidate) { _ in
                if failPrepare { throw Rejected.host }
            }
        }
        router.keyDown(keyCode: 123, modifiers: .shift)
        try session.beginCapture(.left, replacing: 0)
        _ = try session.record(keyCode: 15, modifiers: [])
        try rejects { try session.apply() }
        try check(router.bindings == session.applied && router.heldKeys == [16,37], "Editor failure changed live owner")
        try check(try Data(contentsOf: url) == originalBytes, "Editor prepare failure saved")
        failPrepare = false
        try session.apply()
        try check(capture.events == [event(16,true),event(37,true),event(16,false),event(37,false)] &&
                  router.bindings == session.applied && !session.hasChanges, "Actual editor commit order")
        router.keyDown(keyCode: 15, modifiers: [])
        let before = capture.events
        try session.apply() // no changes: no disk write or flush
        try check(capture.events == before && router.heldKeys == [37], "Editor no-change Apply released key")

        let blocker = directory.appendingPathComponent("editor-blocker")
        try originalBytes.write(to: blocker)
        let failed = NativeHotkeyPreferencesSession(bindings: router.bindings,
            configurationURL: blocker.appendingPathComponent("config.json"),
            prepare: { try router.prepareInstallation($0) })
        try failed.resetDefaults()
        try rejects { try failed.apply() }
        try check(router.heldKeys == [37] && capture.events == before && failed.hasChanges,
                  "Actual editor save failure released input")
    }
    #endif

    static func main() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("RisingLands-HotkeyRouter-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        // Preserve failed fixtures for diagnosis. Successful temporary fixtures only
        // are removed; no shared cache, preferences or another task's data is used.
        try baseline(); print("PASS 1 committed defaults: 20 physical keys x 8 modifier combinations")
        try modifierRelease(); print("PASS 2 modifier release before latched keyUp")
        try aliases(); print("PASS 3 both alias release orders, repeat and duplicate up")
        try repeatsAndOrphans(); print("PASS 4 repeat, duplicate down and up without down")
        try exactPhysicalLatch(); print("PASS 5 exact chords, unmatched physical latch and changed modifiers")
        try focus(); print("PASS 6 inactive ownership, sorted focus flush and no synthetic restore")
        try command(); print("PASS 7 Command menu disposition and earlier latch release")
        try invalidConfiguration(directory); print("PASS 8 invalid/conflicting configuration preserves active state")
        try transactionFailures(directory); print("PASS 9 failed prepare/save preserve file, held aliases and releases")
        try successfulTransaction(directory); print("PASS 10 prepare/save/commit exact injection order and one-shot commit")
        try resetAndNoOp(directory); print("PASS 11 reset/no-op installation without duplicate movement/order packets")
        try replayPayload(); print("PASS 12 deterministic existing recorder payload and event order")
        #if HOTKEY_EDITOR_TESTS
        try editorTransaction(directory); print("PASS 13 real editor model prepare/save/commit adapter; no UI")
        print("HotkeyInputRouterTests: 13 groups passed (including editor adapter); no UI/audio/engine")
        #else
        print("HotkeyInputRouterTests: 12 Foundation-only groups passed; no UI/audio/engine")
        #endif
        try FileManager.default.removeItem(at: directory)
    }
}
