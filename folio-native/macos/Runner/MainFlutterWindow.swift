import Cocoa
import FlutterMacOS

enum FolioKeyboardEvent {
  enum ResponderRole { case window, controller, other, missing }

  static func isQuitShortcut(_ event: NSEvent) -> Bool {
    event.type == .keyDown &&
      event.charactersIgnoringModifiers?.lowercased() == "q" &&
      event.modifierFlags.intersection([.command, .control, .option, .shift]) == .command
  }

  static func shouldRestoreResponder(
    _ role: ResponderRole, gridFocused: Bool, hasAttachedSheet: Bool
  ) -> Bool {
    guard !hasAttachedSheet, role != .controller else { return false }
    return gridFocused || role == .window || role == .missing
  }

  // Accessibility keyboards may supply only AppKit's aggregate modifier bits.
  // Flutter tracks the device-specific left/right bits. Preserve known sides,
  // and use the left side only when the event supplies no side information.
  static func modifierFlags(_ original: NSEvent.ModifierFlags) -> NSEvent.ModifierFlags {
    var flags = original.rawValue
    let modifiers: [(NSEvent.ModifierFlags, UInt, UInt)] = [
      (.shift, 0x2, 0x4), (.control, 0x1, 0x2000),
      (.option, 0x20, 0x40), (.command, 0x8, 0x10),
    ]
    for (aggregate, left, right) in modifiers {
      if flags & aggregate.rawValue != 0 && flags & (left | right) == 0 { flags |= left }
    }
    return NSEvent.ModifierFlags(rawValue: flags)
  }

  static func normalize(_ event: NSEvent) -> NSEvent {
    let flags = Self.modifierFlags(event.modifierFlags)
    guard flags != event.modifierFlags else { return event }
    let modifierEvent = event.type == .flagsChanged
    return NSEvent.keyEvent(
      with: event.type, location: event.locationInWindow, modifierFlags: flags,
      timestamp: event.timestamp, windowNumber: event.windowNumber, context: nil,
      characters: modifierEvent ? "" : event.characters ?? "",
      charactersIgnoringModifiers: modifierEvent ? "" : event.charactersIgnoringModifiers ?? "",
      isARepeat: modifierEvent ? false : event.isARepeat, keyCode: event.keyCode
    ) ?? event
  }

}

// File commands use the shared Dart gate. Edit history is wired separately;
// the existing View/Window menus and other text-responder actions stay intact.
final class FolioFileMenu: NSObject, NSMenuItemValidation {
  let menu = NSMenu(title: "File")
  weak var window: NSWindow?
  var onCommand: ((String, @escaping () -> Void) -> Void)?
  private var available = false
  private var canReplace = false
  private var pending = false
  private var exportItem: NSMenuItem!
  private var pdfItem: NSMenuItem!

  init(window: NSWindow) {
    self.window = window
    super.init()
    add("New Workspace", "newWorkspace", "n")
    add("New from Template…", "fromTemplate", "", [])
    add("New Tab", "newTab", "t")
    add("Open…", "open", "o")
    add("Open in New Tab…", "openInNewTab", "o", [.command, .option])
    add("Recent Files and Recovery…", "recent", "", [])
    menu.addItem(.separator())
    add("Save", "save", "s")
    add("Save As…", "saveAs", "s", [.command, .shift])
    add("Save Workspace Copy…", "saveCopy", "", [])
    add("Save as Template…", "saveTemplate", "", [])
    exportItem = add("Export Word…", "export", "e", [.command, .shift])
    pdfItem = add("Export PDF…", "exportPdf", "", [])
    menu.addItem(.separator())
    add("Compatibility Report", "compatibility", "", [])
    menu.addItem(.separator())
    add("Close Tab", "closeTab", "w", [.command, .option])
  }

  @discardableResult
  private func add(_ title: String, _ command: String, _ key: String,
                   _ modifiers: NSEvent.ModifierFlags = .command) -> NSMenuItem {
    let item = NSMenuItem(title: title, action: #selector(invoke(_:)), keyEquivalent: key)
    item.keyEquivalentModifierMask = modifiers
    item.target = self
    item.representedObject = command
    menu.addItem(item)
    return item
  }

  func update(available: Bool, canReplace: Bool, spreadsheet: Bool) {
    self.available = available
    self.canReplace = canReplace
    exportItem.title = spreadsheet ? "Export Excel…" : "Export Word…"
    pdfItem.isHidden = spreadsheet
    // Dart publishes after the engine is ready, by which time the nib menu is installed.
    if menu.supermenu == nil, let main = NSApp.mainMenu {
      let item = NSMenuItem(title: "File", action: nil, keyEquivalent: "")
      item.submenu = menu
      main.insertItem(item, at: min(1, main.numberOfItems))
    }
    menu.update()
  }

  func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
    guard available, !pending, let window = window,
          window.isVisible, window.attachedSheet == nil,
          let command = menuItem.representedObject as? String else { return false }
    return canReplace || !["open", "newWorkspace", "recent", "fromTemplate"].contains(command)
  }

  @objc private func invoke(_ sender: NSMenuItem) {
    guard validateMenuItem(sender), let command = sender.representedObject as? String,
          let onCommand = onCommand else { return }
    pending = true
    menu.update()
    onCommand(command) { [weak self] in
      self?.pending = false
      self?.menu.update()
    }
  }
}

// Native history uses the same owner as Dart shortcuts: the focused draft or
// the committed workspace. Attached AppKit panels retain their responder chain.
final class FolioHistoryMenu: NSObject, NSMenuItemValidation {
  weak var window: NSWindow?
  var onCommand: ((String, Int) -> Void)?
  private var items: [String: NSMenuItem] = [:]
  private var available = false
  private var canUndo = false
  private var canRedo = false
  private var generation = 0
  private var observers: [NSObjectProtocol] = []

  init(window: NSWindow) {
    self.window = window
    super.init()
    for name in [NSWindow.willBeginSheetNotification, NSWindow.didEndSheetNotification] {
      observers.append(NotificationCenter.default.addObserver(forName: name, object: window, queue: .main) { [weak self] _ in
        self?.refresh()
        DispatchQueue.main.async { [weak self] in self?.refresh() }
      })
    }
  }

  deinit { for observer in observers { NotificationCenter.default.removeObserver(observer) } }

  func update(available: Bool, canUndo: Bool, canRedo: Bool, generation: Int) {
    self.available = available
    self.canUndo = canUndo
    self.canRedo = canRedo
    self.generation = generation
    refresh()
  }

  private var nativePanel: Bool {
    window?.attachedSheet != nil || (NSApp.modalWindow != nil && NSApp.modalWindow !== window)
  }

  func refresh() {
    if items.isEmpty, let main = NSApp.mainMenu {
      for top in main.items {
        for item in top.submenu?.items ?? [] {
          for command in ["undo", "redo"] where item.action == NSSelectorFromString(command + ":") && item.target == nil {
            items[command] = item
          }
        }
      }
    }
    for (command, item) in items {
      item.action = nativePanel ? NSSelectorFromString(command + ":") : #selector(invoke(_:))
      item.target = nativePanel ? nil : self
      item.representedObject = command
    }
    items["undo"]?.menu?.update()
  }

  func validateMenuItem(_ item: NSMenuItem) -> Bool {
    guard available, !nativePanel, window?.isVisible == true,
          let command = item.representedObject as? String else { return false }
    return command == "undo" ? canUndo : command == "redo" && canRedo
  }

  @objc private func invoke(_ sender: NSMenuItem) {
    guard validateMenuItem(sender), let command = sender.representedObject as? String else { return }
    onCommand?(command, generation)
  }

  func performKeyEquivalent(with event: NSEvent) -> Bool {
    refresh()
    guard !nativePanel, event.type == .keyDown,
          event.charactersIgnoringModifiers?.lowercased() == "z" else { return false }
    let flags = event.modifierFlags.intersection([.command, .control, .option, .shift])
    guard flags == .command || flags == [.command, .shift] else { return false }
    let command = flags.contains(.shift) ? "redo" : "undo"
    guard let item = items[command], validateMenuItem(item) else { return false }
    invoke(item)
    return true
  }
}

class FolioFlutterViewController: FlutterViewController {
  override func keyDown(with event: NSEvent) { super.keyDown(with: FolioKeyboardEvent.normalize(event)) }
  override func keyUp(with event: NSEvent) { super.keyUp(with: FolioKeyboardEvent.normalize(event)) }
  override func flagsChanged(with event: NSEvent) { super.flagsChanged(with: FolioKeyboardEvent.normalize(event)) }
}

class MainFlutterWindow: NSWindow {
  static weak var current: MainFlutterWindow?
  private var lifecycle: FlutterMethodChannel?
  private var keyboard: FlutterMethodChannel?
  private var fileCommands: FlutterMethodChannel?
  private var fileAccess: FlutterMethodChannel?
  private var fileLeases: [String: (URL, Bool)] = [:]
  private var fileMenu: FolioFileMenu?
  private var historyMenu: FolioHistoryMenu?
  private var fonts: FlutterMethodChannel?
  private var clipboard: FlutterMethodChannel?
  private var textSort: FlutterMethodChannel?
  private var activationRecovery: FolioActivationRecovery?
  private var gridFocused = false
  private var closing = false

  override func awakeFromNib() {
    let controller = FolioFlutterViewController()
    contentViewController = controller
    setContentSize(NSSize(width: 1440, height: 960))
    // D01.09: the window reopens where it was left.
    setFrameAutosaveName("FolioMainWindow")
    minSize = NSSize(width: 1150, height: 720)
    center()
    title = "Folio Native"
    titleVisibility = .hidden
    titlebarAppearsTransparent = true
    styleMask.insert(.fullSizeContentView)
    isMovableByWindowBackground = false
    RegisterGeneratedPlugins(registry: controller)
    activationRecovery = FolioActivationRecovery(
      window: self,
      state: { [weak self] in
        .init(active: NSApp.isActive, hidden: NSApp.isHidden,
              visibleWindow: self?.isVisible ?? false,
              minimized: self?.isMiniaturized ?? true,
              keyWindow: self?.isKeyWindow ?? false)
      },
      resume: { [weak controller] in
        controller?.engine.binaryMessenger.send(
          onChannel: "flutter/lifecycle",
          message: "AppLifecycleState.resumed".data(using: .utf8))
      })
    lifecycle = FlutterMethodChannel(name: "folio/lifecycle", binaryMessenger: controller.engine.binaryMessenger)
    keyboard = FlutterMethodChannel(name: "folio/keyboard", binaryMessenger: controller.engine.binaryMessenger)
    let fileChannel = FlutterMethodChannel(name: "folio/file-menu", binaryMessenger: controller.engine.binaryMessenger)
    let accessChannel = FlutterMethodChannel(name: "folio/file-access", binaryMessenger: controller.engine.binaryMessenger)
    fileAccess = accessChannel
    accessChannel.setMethodCallHandler { [weak self] call, result in
      guard let self = self, let args = call.arguments as? [String: Any] else {
        result(FlutterMethodNotImplemented); return
      }
      do {
        switch call.method {
        case "capture":
          guard let path = args["path"] as? String else { throw CocoaError(.fileReadInvalidFileName) }
          let url = URL(fileURLWithPath: path)
          let bookmark = try url.bookmarkData(options: [.withSecurityScope],
                                             includingResourceValuesForKeys: nil, relativeTo: nil)
          result(bookmark.base64EncodedString())
        case "acquire":
          guard let encoded = args["bookmark"] as? String, let data = Data(base64Encoded: encoded)
          else { throw CocoaError(.fileReadCorruptFile) }
          var stale = false
          let url = try URL(resolvingBookmarkData: data, options: [.withSecurityScope, .withoutUI],
                            relativeTo: nil, bookmarkDataIsStale: &stale)
          let started = url.startAccessingSecurityScopedResource()
          do {
            let bookmark = try url.bookmarkData(options: [.withSecurityScope],
                                               includingResourceValuesForKeys: nil, relativeTo: nil)
            let lease = UUID().uuidString
            self.fileLeases[lease] = (url, started)
            result(["path": url.path, "bookmark": bookmark.base64EncodedString(), "lease": lease])
          } catch {
            if started { url.stopAccessingSecurityScopedResource() }
            throw error
          }
        case "release":
          if let lease = args["lease"] as? String, let (url, started) = self.fileLeases.removeValue(forKey: lease), started {
            url.stopAccessingSecurityScopedResource()
          }
          result(nil)
        default: result(FlutterMethodNotImplemented)
        }
      } catch {
        result(FlutterError(code: "file-access", message: "The file could not be accessed. Choose Save As to select a location. \(error.localizedDescription)", details: nil))
      }
    }
    fileCommands = fileChannel
    fileMenu = FolioFileMenu(window: self)
    fileMenu?.onCommand = { command, done in
      fileChannel.invokeMethod("command", arguments: command) { _ in done() }
    }
    fileChannel.setMethodCallHandler { [weak self] call, result in
      guard call.method == "state", let args = call.arguments as? [String: Any] else {
        result(FlutterMethodNotImplemented); return
      }
      self?.fileMenu?.update(available: args["available"] as? Bool ?? false,
                            canReplace: args["canReplace"] as? Bool ?? false,
                            spreadsheet: args["spreadsheet"] as? Bool ?? false)
      result(nil)
    }
    let historyChannel = FlutterMethodChannel(name: "folio/history-menu", binaryMessenger: controller.engine.binaryMessenger)
    historyMenu = FolioHistoryMenu(window: self)
    historyMenu?.onCommand = { command, generation in
      historyChannel.invokeMethod("command", arguments: ["command": command, "generation": generation])
    }
    historyChannel.setMethodCallHandler { [weak self] call, result in
      guard call.method == "state", let args = call.arguments as? [String: Any] else {
        result(FlutterMethodNotImplemented); return
      }
      self?.historyMenu?.update(available: args["available"] as? Bool ?? false,
                               canUndo: args["canUndo"] as? Bool ?? false,
                               canRedo: args["canRedo"] as? Bool ?? false,
                               generation: args["generation"] as? Int ?? 0)
      result(nil)
    }
    fonts = FlutterMethodChannel(name: "folio/fonts", binaryMessenger: controller.engine.binaryMessenger)
    textSort = FlutterMethodChannel(name: "folio/text-sort", binaryMessenger: controller.engine.binaryMessenger)
    textSort?.setMethodCallHandler { call, result in
      guard call.method == "ranks" else { result(FlutterMethodNotImplemented); return }
      guard let args = call.arguments as? [String: Any],
            let values = args["values"] as? [[String]], values.count <= 3,
            values.allSatisfy({ $0.count <= 1000 }),
            values.flatMap({ $0 }).reduce(0, { $0 + $1.utf8.count }) <= 8_000_000,
            let locale = args["locale"] as? String, locale.count <= 100,
            let sensitive = args["caseSensitive"] as? Bool else {
        result(FlutterError(code: "sort", message: "The sort settings or table are too large.", details: nil)); return
      }
      DispatchQueue.global(qos: .userInitiated).async {
        let ranks = FolioTextSort.ranks(values, locale: locale, caseSensitive: sensitive)
        DispatchQueue.main.async { result(ranks) }
      }
    }
    clipboard = FlutterMethodChannel(name: "folio/clipboard", binaryMessenger: controller.engine.binaryMessenger)
    clipboard?.setMethodCallHandler { call, result in
      let board = NSPasteboard.general
      let cellType = NSPasteboard.PasteboardType("com.folio.native.cells-token")
      switch call.method {
      case "write":
        guard let args = call.arguments as? [String: Any], let text = args["text"] as? String,
              let token = args["token"] as? String, text.utf8.count <= 32_000_000, token.utf8.count <= 128 else {
          result(FlutterError(code: "clipboard", message: "The copied range is too large.", details: nil)); return
        }
        let item = NSPasteboardItem()
        item.setString(text, forType: .string)
        item.setString(token, forType: cellType)
        board.clearContents()
        guard board.writeObjects([item]) else {
          result(FlutterError(code: "clipboard", message: "The cells could not be copied. Try again.", details: nil)); return
        }
        result(nil)
      case "read":
        let change = board.changeCount
        guard let text = board.string(forType: .string) else { result(nil); return }
        guard text.utf8.count <= 32_000_000 else {
          result(FlutterError(code: "clipboard", message: "Clipboard text exceeds 32 MB.", details: nil)); return
        }
        var data: [String: Any] = ["text": text]
        if let token = board.string(forType: cellType) { data["token"] = token }
        guard change == board.changeCount else {
          result(FlutterError(code: "clipboard", message: "The clipboard changed. Paste again.", details: nil)); return
        }
        result(data)
      default: result(FlutterMethodNotImplemented)
      }
    }
    fonts?.setMethodCallHandler { call, result in
      guard call.method == "families" else { result(FlutterMethodNotImplemented); return }
      result(NSFontManager.shared.availableFontFamilies.sorted { $0.localizedCaseInsensitiveCompare($1) == .orderedAscending })
    }
    keyboard?.setMethodCallHandler { [weak self] call, result in
      guard call.method == "setGridFocused", let focused = call.arguments as? Bool else {
        result(FlutterMethodNotImplemented)
        return
      }
      self?.gridFocused = focused
      self?.restoreFlutterResponder()
      result(nil)
    }
    MainFlutterWindow.current = self
    super.awakeFromNib()
  }

  private func restoreFlutterResponder() {
    // With accessibility enabled, AppKit may reactivate Flutter's old text
    // responder after its editing client has gone away. The grid has no text
    // client: route through its public view controller while Dart owns focus.
    // Removing a dialog's accessible text field can also leave the window
    // itself as first responder, bypassing Flutter entirely. Recover that empty
    // responder state for document input as well; keep active field editors and
    // attached native panels on their existing input route.
    guard let controller = contentViewController as? FlutterViewController else { return }
    let role: FolioKeyboardEvent.ResponderRole = firstResponder == nil ? .missing
      : firstResponder === controller ? .controller
      : firstResponder === self ? .window : .other
    guard FolioKeyboardEvent.shouldRestoreResponder(
      role, gridFocused: gridFocused, hasAttachedSheet: attachedSheet != nil
    ) else { return }
    makeFirstResponder(controller)
  }

  override func performKeyEquivalent(with event: NSEvent) -> Bool {
    restoreFlutterResponder()
    // File commands belong to the workspace even when a text client is active.
    // Disabled items fall through; Dart also rejects commands behind a dialog.
    if fileMenu?.menu.performKeyEquivalent(with: event) == true { return true }
    if historyMenu?.performKeyEquivalent(with: event) == true { return true }
    // The text client can be replaced while asynchronous cell entry advances.
    // Route native Quit before Flutter's asynchronous key/text dispatch so that
    // the outgoing editor cannot swallow it. AppKit still validates the menu.
    if FolioKeyboardEvent.isQuitShortcut(event),
       NSApp.mainMenu?.performKeyEquivalent(with: event) == true {
      return true
    }
    return super.performKeyEquivalent(with: event)
  }

  override func sendEvent(_ event: NSEvent) {
    if event.type == .keyDown || event.type == .keyUp || event.type == .flagsChanged {
      restoreFlutterResponder()
    }
    super.sendEvent(event)
  }

  func prepareClose(_ completion: @escaping (Bool) -> Void) {
    if closing { completion(true); return }
    guard let lifecycle = lifecycle else { completion(true); return }
    lifecycle.invokeMethod("prepareClose", arguments: nil) { result in
      if let error = result as? FlutterError {
        let alert = NSAlert()
        alert.messageText = "Folio could not finish saving"
        alert.informativeText = error.message ?? "Please save a copy before closing."
        alert.runModal()
        completion(false)
      } else {
        completion((result as? Bool) == true)
      }
    }
  }

  override func close() {
    if closing { super.close(); return }
    prepareClose { [weak self] success in
      guard success, let self = self else { return }
      self.closing = true
      self.close()
    }
  }
}
