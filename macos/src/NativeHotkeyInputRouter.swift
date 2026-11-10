import Foundation

/// Physical input lifetime only. The host retains menu dispatch, mouse mapping,
/// frame stamping, recording and engine injection. Call on the main actor;
/// the synchronous, infallible sink must not reenter this owner.
@MainActor
final class NativeHotkeyInputRouter {
    typealias Bindings = NativeHotkeyBindings
    typealias Modifiers = Bindings.Modifiers
    typealias Commit = @MainActor () -> Void

    struct KeyEvent: Equatable, Sendable {
        let virtualKey: Int
        let down: Bool

        /// The existing NativeMetalGameView.inputEvent payload, without a frame.
        var payload: [String: Any] {
            ["kind": "key", "vkey": virtualKey, "down": down]
        }
    }

    enum Disposition: Equatable {
        case handled, unhandled, nativeMenu
    }

    private struct Press {
        // Remember unmatched downs too: a modifier change cannot reinterpret one.
        let action: Bindings.Action?
    }

    private(set) var bindings: Bindings
    private(set) var isActive = false
    private(set) var heldKeys = Set<Int>()
    private var presses: [UInt16: Press] = [:]
    private var references: [Bindings.Action: Int] = [:]
    private let sink: @MainActor (KeyEvent) -> Void

    init(bindings: Bindings = Bindings(), sink: @escaping @MainActor (KeyEvent) -> Void) {
        self.bindings = bindings
        self.sink = sink
    }

    /// Activate only for the active game window with the game view as responder.
    /// Deactivation flushes before the host detaches/replaces the old input sink.
    /// Reactivation never synthesizes downs for keys still physically held.
    func setActive(_ active: Bool) {
        if !active { releaseAllHeldKeys() }
        isActive = active
    }

    @discardableResult
    func keyDown(keyCode: UInt16, modifiers: Modifiers, isRepeat: Bool = false) -> Disposition {
        // Never steal even an unassigned Command shortcut from the native host.
        guard !modifiers.contains(.command) else { return .nativeMenu }
        guard isActive else { return .unhandled }
        if let press = presses[keyCode] {
            return press.action == nil ? .unhandled : .handled
        }
        // Includes repeats arriving after focus loss or a successful rebind flush.
        guard !isRepeat else { return .unhandled }
        synchronizeModifiers(modifiers)
        let action = bindings.action(keyCode: keyCode, modifiers: modifiers)
        presses[keyCode] = Press(action: action)
        guard let action else { return .unhandled }
        let count = references[action, default: 0]
        references[action] = count + 1
        if count == 0 { emit(action.virtualKey, down: true) }
        return .handled
    }

    @discardableResult
    func keyUp(keyCode: UInt16, modifiers: Modifiers) -> Disposition {
        guard isActive else {
            return modifiers.contains(.command) ? .nativeMenu : .unhandled
        }
        // Preserve the committed host's modifier-before-key order. Command must
        // not prevent releasing an earlier gameplay down with a different chord.
        synchronizeModifiers(modifiers)
        let action = presses.removeValue(forKey: keyCode)?.action
        if let action {
            let count = references[action, default: 0]
            if count > 1 {
                references[action] = count - 1
            } else {
                references.removeValue(forKey: action)
                emit(action.virtualKey, down: false)
            }
        }
        if modifiers.contains(.command) { return .nativeMenu }
        return action == nil ? .unhandled : .handled
    }

    /// Call for flagsChanged and before the existing mouse event callback.
    /// Command itself has no gameplay channel. Shift/Control/Option remain
    /// independent channels, including when no action chord matches.
    func synchronizeModifiers(_ modifiers: Modifiers) {
        guard isActive else { return }
        for (flag, key): (Modifiers, Int) in [(.shift, 16), (.control, 17), (.option, 18)] {
            let down = modifiers.contains(flag)
            if down != heldKeys.contains(key) { emit(key, down: down) }
        }
    }

    /// Idempotent, deterministic release through the SAME sink as normal keys.
    /// Also clears unmatched physical downs and alias counts. No restore downs.
    func releaseAllHeldKeys() {
        presses.removeAll(keepingCapacity: true)
        references.removeAll(keepingCapacity: true)
        for key in heldKeys.sorted() { emit(key, down: false) }
    }

    /// Directly compatible with NativeHotkeyPreferencesSession.Prepare:
    /// prepare -> candidate.save(to:) -> returned synchronous commit.
    /// Bindings is already validated by its value-type API. All additional host
    /// checks/allocations belong in checkHost and MUST be free of side effects.
    /// On a throw, or if save fails and this closure is discarded, NOTHING here
    /// changes. Serialize the entire transaction; do not retain it across an
    /// await, other installation, sink replacement, or editor lifetime.
    func prepareInstallation(_ candidate: Bindings,
                             checkHost: (Bindings) throws -> Void = { _ in }) throws -> Commit {
        try checkHost(candidate)
        var committed = false
        return {
            guard !committed else { return }
            committed = true
            guard self.bindings != candidate else { return }
            self.releaseAllHeldKeys()
            self.bindings = candidate
        }
    }

    private func emit(_ key: Int, down: Bool) {
        if down { heldKeys.insert(key) } else { heldKeys.remove(key) }
        sink(KeyEvent(virtualKey: key, down: down))
    }
}
