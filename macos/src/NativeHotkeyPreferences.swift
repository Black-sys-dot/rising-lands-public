import AppKit

/// A main-thread draft. Construction and all model operations require no window.
/// The host prepares without mutation, then supplies an infallible installation.
@MainActor
final class NativeHotkeyPreferencesSession {
    typealias Bindings = NativeHotkeyBindings
    typealias Commit = @MainActor () -> Void
    typealias Prepare = @MainActor (Bindings) throws -> Commit

    struct Capture: Equatable {
        let action: Bindings.Action
        /// nil appends an alternative; an index replaces only that alternative.
        let aliasIndex: Int?
    }

    enum EditorError: Error, LocalizedError {
        case invalidSelection, finishCapture, busy
        var errorDescription: String? {
            switch self {
            case .invalidSelection: return "Selecione um atalho da lista."
            case .finishCapture: return "Conclua ou cancele a captura antes de aplicar."
            case .busy: return "Aguarde a aplicação dos atalhos."
            }
        }
    }

    private(set) var applied: Bindings
    private(set) var draft: Bindings
    private(set) var capture: Capture?
    private(set) var isApplying = false
    let configurationURL: URL
    private let prepare: Prepare
    var hasChanges: Bool { draft != applied }

    init(bindings: Bindings, configurationURL: URL, prepare: @escaping Prepare) {
        applied = bindings
        draft = bindings
        self.configurationURL = configurationURL
        self.prepare = prepare
    }

    func beginCapture(_ action: Bindings.Action, replacing aliasIndex: Int?) throws {
        try checkIdle()
        if let index = aliasIndex, !draft.chords(for: action).indices.contains(index) {
            throw EditorError.invalidSelection
        }
        capture = Capture(action: action, aliasIndex: aliasIndex)
    }

    /// Failed validation retains the capture and draft so the user can retry.
    @discardableResult
    func record(keyCode: UInt16, modifiers: Bindings.Modifiers, isRepeat: Bool = false) throws -> Bool {
        try checkIdle()
        guard let target = capture, !isRepeat else { return false }
        var aliases = draft.chords(for: target.action)
        let chord = Bindings.Chord(keyCode: keyCode, modifiers: modifiers)
        if let index = target.aliasIndex { aliases[index] = chord }
        else { aliases.append(chord) }
        try draft.rebind(target.action, to: aliases)
        capture = nil
        return true
    }

    func cancelCapture() { if !isApplying { capture = nil } }

    func removeAlias(_ action: Bindings.Action, at index: Int) throws {
        try checkIdle()
        guard draft.chords(for: action).indices.contains(index) else { throw EditorError.invalidSelection }
        var aliases = draft.chords(for: action)
        aliases.remove(at: index)
        try draft.rebind(action, to: aliases)
        capture = nil
    }

    func resetDefaults() throws {
        try checkIdle()
        draft.resetToDefaults()
        capture = nil
    }

    func cancel() {
        guard !isApplying else { return }
        draft = applied
        capture = nil
    }

    /// No throwing operation follows the atomic save. A throwing prepare MUST NOT
    /// change live state or files. Commit MUST NOT fail, throw, schedule a fallible
    /// install, or reenter this editor. See the host contract in the documentation.
    func apply() throws {
        try checkIdle()
        guard capture == nil else { throw EditorError.finishCapture }
        guard hasChanges else { return }
        isApplying = true
        defer { isApplying = false }
        let candidate = draft
        let commit = try prepare(candidate)
        try candidate.save(to: configurationURL)
        commit()
        applied = candidate
    }

    private func checkIdle() throws {
        if isApplying { throw EditorError.busy }
    }

    /// UI errors deliberately omit physical codes, file paths and host internals.
    static func message(for error: Error) -> String {
        if let error = error as? EditorError { return error.errorDescription! }
        if let error = error as? Bindings.BindingError {
            switch error {
            case .reservedForMenu: return "Combinações com Command são reservadas aos menus do app."
            case .conflict(let first, let second):
                return first == second
                    ? "Esse atalho já está na lista desta ação."
                    : "Conflito de atalhos entre \(first.labelPTBR) e \(second.labelPTBR). Escolha outra combinação."
            case .invalidKeyCode, .invalidModifiers: return "Esta tecla ou combinação não pode ser usada. Tente outra."
            case .writeFailed, .nonFileURL: return "Não foi possível salvar os atalhos. As alterações não foram aplicadas."
            default: return "Não foi possível usar estes atalhos. As alterações não foram aplicadas."
            }
        }
        return "Não foi possível aplicar os atalhos. As alterações não foram aplicadas."
    }
}

/// Lazily creates its own preferences window. Never installs event monitors or
/// shows a window at initialization; the host explicitly calls showWindow(_:).
@MainActor
final class NativeHotkeyPreferencesController: NSWindowController, NSWindowDelegate,
                                                NSTableViewDataSource, NSTableViewDelegate {
    typealias Bindings = NativeHotkeyBindings
    /// Must resolve physical positions using the current keyboard layout. nil is
    /// displayed as an unnamed key, never as a guessed US character or raw code.
    typealias KeyLabel = @MainActor (UInt16) -> String?
    let session: NativeHotkeyPreferencesSession
    private let keyLabel: KeyLabel
    private let table = NSTableView()
    private let alternatives = NSPopUpButton(frame: .zero, pullsDown: false)
    private let feedback = NSTextField(wrappingLabelWithString: "")
    private let recorder = NativeHotkeyPreferencesRecorder(frame: .zero)
    private let change = NSButton(title: "Alterar", target: nil, action: nil)
    private let add = NSButton(title: "Adicionar alternativa", target: nil, action: nil)
    private let remove = NSButton(title: "Remover", target: nil, action: nil)
    private let cancelRecording = NSButton(title: "Cancelar captura", target: nil, action: nil)
    private let reset = NSButton(title: "Restaurar padrões", target: nil, action: nil)
    private let applyButton = NSButton(title: "Aplicar", target: nil, action: nil)
    private let cancelButton = NSButton(title: "Cancelar", target: nil, action: nil)
    private var isRefreshing = false

    init(bindings: Bindings, configurationURL: URL, keyLabel: @escaping KeyLabel,
         prepare: @escaping NativeHotkeyPreferencesSession.Prepare) {
        session = NativeHotkeyPreferencesSession(bindings: bindings, configurationURL: configurationURL,
                                                prepare: prepare)
        self.keyLabel = keyLabel
        super.init(window: nil)
    }

    required init?(coder: NSCoder) { nil }

    override func showWindow(_ sender: Any?) {
        super.showWindow(sender)
        // Reuse the window after Cancel/close, but never reuse its discarded
        // table cells or popup labels. An active capture keeps its prompt.
        if session.capture == nil {
            refresh(message: "Escolha uma ação para editar seus atalhos.")
        }
    }

    // NSWindowController treats a nil nib name as already loaded, even with a
    // nil window. Supply a lazy-load identity; loadWindow below builds the view
    // entirely in code and never asks AppKit to read a nib resource.
    override var windowNibName: NSNib.Name? { NSNib.Name("NativeHotkeyPreferences") }

    override func loadWindow() {
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 780, height: 700),
                              styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        window.title = "Atalhos de teclado"
        window.minSize = NSSize(width: 720, height: 650)
        window.isReleasedWhenClosed = false
        window.delegate = self
        let content = NSView()
        window.contentView = content
        self.window = window

        let title = NSTextField(labelWithString: "Atalhos de teclado")
        title.font = .boldSystemFont(ofSize: 20)
        let explanation = NSTextField(wrappingLabelWithString:
            "Selecione uma ação para editar seus atalhos. As alterações só entram em vigor ao aplicar. Command fica reservado aos menus do app.")
        let actionColumn = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("action"))
        actionColumn.title = "Ação"
        actionColumn.width = 200
        let chordColumn = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("shortcuts"))
        chordColumn.title = "Atalhos atuais"
        chordColumn.width = 480
        table.addTableColumn(actionColumn)
        table.addTableColumn(chordColumn)
        table.dataSource = self
        table.delegate = self
        table.rowHeight = 28
        table.allowsEmptySelection = false
        table.allowsMultipleSelection = false
        table.setAccessibilityLabel("Ações e atalhos de teclado")
        let scroll = NSScrollView()
        scroll.documentView = table
        scroll.hasVerticalScroller = true
        scroll.borderType = .bezelBorder
        scroll.heightAnchor.constraint(greaterThanOrEqualToConstant: 210).isActive = true

        alternatives.target = self
        alternatives.action = #selector(alternativeChanged)
        alternatives.setAccessibilityLabel("Atalho da ação selecionada")
        configure(change, #selector(changeShortcut))
        configure(add, #selector(addShortcut))
        configure(remove, #selector(removeShortcut))
        configure(cancelRecording, #selector(cancelCapture))
        configure(reset, #selector(resetDefaults))
        configure(applyButton, #selector(applyChanges))
        configure(cancelButton, #selector(cancelChanges))
        // No Return/Escape key equivalents: both are legitimate captured keys.
        recorder.controller = self
        recorder.heightAnchor.constraint(equalToConstant: 30).isActive = true
        feedback.setAccessibilityLabel("Resultado da edição de atalhos")
        feedback.heightAnchor.constraint(greaterThanOrEqualToConstant: 40).isActive = true
        let editing = NSStackView(views: [alternatives, change, add, remove])
        editing.orientation = .horizontal
        let recording = NSStackView(views: [recorder, cancelRecording])
        recording.orientation = .horizontal
        recorder.widthAnchor.constraint(greaterThanOrEqualToConstant: 390).isActive = true
        let spacer = NSView()
        let buttons = NSStackView(views: [reset, spacer, cancelButton, applyButton])
        buttons.orientation = .horizontal
        spacer.setContentHuggingPriority(.defaultLow, for: .horizontal)
        let stack = NSStackView(views: [title, explanation, scroll, editing, recording, feedback, buttons])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 12
        stack.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),
            stack.topAnchor.constraint(equalTo: content.topAnchor, constant: 20),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -20)
        ])
        for view in [explanation, scroll, editing, recording, feedback, buttons] {
            view.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        }
        table.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
        refresh(message: "Escolha uma ação. Atalhos com modificadores opcionais aceitam Shift, Control e Option.")
    }

    func numberOfRows(in tableView: NSTableView) -> Int { Bindings.Action.allCases.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard Bindings.Action.allCases.indices.contains(row) else { return nil }
        let action = Bindings.Action.allCases[row]
        let text = tableColumn?.identifier.rawValue == "action" ? action.labelPTBR : shortcutSummary(for: action)
        let label = NSTextField(labelWithString: text)
        label.lineBreakMode = .byTruncatingTail
        label.toolTip = text
        return label
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        guard !isRefreshing else { return }
        session.cancelCapture()
        refresh(message: "Escolha Alterar ou Adicionar alternativa e pressione a combinação desejada.")
    }

    func shortcutSummary(for action: Bindings.Action) -> String {
        let chords = session.draft.chords(for: action)
        return chords.isEmpty ? "Sem atalho" : chords.map(display).joined(separator: " ou ")
    }

    func display(_ chord: Bindings.Chord) -> String {
        var parts: [String] = []
        if chord.modifiers.contains(.control) { parts.append("Control") }
        if chord.modifiers.contains(.option) { parts.append("Option") }
        if chord.modifiers.contains(.shift) { parts.append("Shift") }
        parts.append(keyLabel(chord.keyCode).flatMap { $0.isEmpty ? nil : $0 } ?? "Tecla sem nome")
        return parts.joined(separator: " + ")
            + (chord.allowsAdditionalGameplayModifiers ? " (modificadores opcionais)" : "")
    }

    /// Call after a keyboard-layout change; affects display only, never bindings.
    func refreshKeyLabels() {
        guard isWindowLoaded else { return }
        refresh(message: "Os nomes das teclas foram atualizados.")
    }

    private var selectedAction: Bindings.Action? {
        Bindings.Action.allCases.indices.contains(table.selectedRow) ? Bindings.Action.allCases[table.selectedRow] : nil
    }

    private func configure(_ button: NSButton, _ selector: Selector) {
        button.target = self
        button.action = selector
        button.bezelStyle = .rounded
    }

    private func refresh(message: String) {
        guard !isRefreshing else { return }
        isRefreshing = true
        defer { isRefreshing = false }
        let selectedRow = table.selectedRow
        let selectedAlias = alternatives.indexOfSelectedItem
        table.reloadData()
        let row = Bindings.Action.allCases.indices.contains(selectedRow) ? selectedRow : 0
        table.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
        alternatives.removeAllItems()
        let aliases = selectedAction.map { session.draft.chords(for: $0) } ?? []
        for (index, chord) in aliases.enumerated() {
            alternatives.addItem(withTitle: "\(index + 1). \(display(chord))")
        }
        if aliases.isEmpty { alternatives.addItem(withTitle: "Sem atalho") }
        if aliases.indices.contains(selectedAlias) { alternatives.selectItem(at: selectedAlias) }
        let capturing = session.capture != nil
        alternatives.isEnabled = !capturing && !aliases.isEmpty
        change.isEnabled = !capturing && !aliases.isEmpty
        add.isEnabled = !capturing && selectedAction != nil
        remove.isEnabled = !capturing && !aliases.isEmpty
        cancelRecording.isEnabled = capturing
        reset.isEnabled = !capturing
        applyButton.isEnabled = !capturing && session.hasChanges
        recorder.needsDisplay = true
        feedback.stringValue = message
        feedback.textColor = .labelColor
    }

    @objc private func alternativeChanged() { session.cancelCapture() }
    @objc func changeShortcut() { beginCapture(replacing: alternatives.indexOfSelectedItem) }
    @objc func addShortcut() { beginCapture(replacing: nil) }

    private func beginCapture(replacing index: Int?) {
        guard let action = selectedAction else { return }
        do {
            try session.beginCapture(action, replacing: index)
            refresh(message: "Pressione a nova combinação para \(action.labelPTBR). Use Cancelar captura para sair.")
            window?.makeFirstResponder(recorder)
        } catch { show(error) }
    }

    @objc func removeShortcut() {
        guard let action = selectedAction else { return }
        do {
            try session.removeAlias(action, at: alternatives.indexOfSelectedItem)
            refresh(message: "Atalho removido do rascunho. Clique em Aplicar para confirmar.")
        } catch { show(error) }
    }

    @objc func cancelCapture() {
        session.cancelCapture()
        refresh(message: "Captura cancelada.")
        window?.makeFirstResponder(table)
    }

    @objc func resetDefaults() {
        do {
            try session.resetDefaults()
            refresh(message: "Padrões restaurados no rascunho. Clique em Aplicar para confirmar.")
        } catch { show(error) }
    }

    @objc func applyChanges() {
        do {
            try session.apply()
            refresh(message: "Atalhos aplicados e salvos.")
        } catch { show(error) }
    }

    @objc func cancelChanges() {
        session.cancel()
        close()
    }

    override func close() {
        session.cancel()
        super.close()
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        session.cancel()
        return true
    }

    func windowWillClose(_ notification: Notification) { session.cancel() }

    func windowDidResignKey(_ notification: Notification) {
        if session.capture != nil { cancelCapture() }
    }

    fileprivate func record(_ event: NSEvent) {
        guard session.capture != nil else { return }
        var modifiers: Bindings.Modifiers = []
        if event.modifierFlags.contains(.shift) { modifiers.insert(.shift) }
        if event.modifierFlags.contains(.control) { modifiers.insert(.control) }
        if event.modifierFlags.contains(.option) { modifiers.insert(.option) }
        if event.modifierFlags.contains(.command) { modifiers.insert(.command) }
        do {
            if try session.record(keyCode: event.keyCode, modifiers: modifiers, isRepeat: event.isARepeat) {
                refresh(message: "Atalho alterado no rascunho. Clique em Aplicar para confirmar.")
                window?.makeFirstResponder(table)
            }
        } catch { show(error) }
    }

    fileprivate func captureLostFocus() {
        if session.capture != nil {
            session.cancelCapture()
            refresh(message: "Captura cancelada ao sair do campo de captura.")
        }
    }

    private func show(_ error: Error) {
        feedback.stringValue = NativeHotkeyPreferencesSession.message(for: error)
        feedback.textColor = .systemRed
    }
}

/// Local responder only: the host must route gameplay exclusively to its game
/// window/view, never through a global/local event monitor while editing.
@MainActor
private final class NativeHotkeyPreferencesRecorder: NSView {
    weak var controller: NativeHotkeyPreferencesController?
    override var acceptsFirstResponder: Bool { true }
    override func keyDown(with event: NSEvent) { controller?.record(event) }
    override func keyUp(with event: NSEvent) {} // Captured events never reach gameplay.
    override func flagsChanged(with event: NSEvent) {} // Modifier-only input is not a chord.
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        // Always leave Command to NSApplication's menu/responder dispatch.
        guard !event.modifierFlags.contains(.command), controller?.session.capture != nil else { return false }
        controller?.record(event)
        return true
    }
    override func resignFirstResponder() -> Bool {
        let result = super.resignFirstResponder()
        if result { controller?.captureLostFocus() }
        return result
    }
    override func draw(_ dirtyRect: NSRect) {
        NSColor.controlBackgroundColor.setFill()
        bounds.fill()
        let active = controller?.session.capture != nil
        let text = active ? "Aguardando a nova combinação…" : "Captura de atalho"
        (text as NSString).draw(at: NSPoint(x: 8, y: 6), withAttributes: [
            .font: NSFont.systemFont(ofSize: 13), .foregroundColor: NSColor.labelColor
        ])
    }
    override func accessibilityLabel() -> String? { "Captura de atalho" }
}
