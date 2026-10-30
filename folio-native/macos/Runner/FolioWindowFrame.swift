import Cocoa

enum FolioWindowFrame {
  // D01.09: reopen where the window was left. AppKit's own restore moves a
  // frame saved on another display onto the screen that has focus, scaled to
  // fit, so apply the saved frame directly while it is still on a screen.
  // Centre only on a first launch.
  static func restore(_ window: NSWindow, name: NSWindow.FrameAutosaveName,
                      defaults: UserDefaults = .standard) {
    let saved = defaults.string(forKey: "NSWindow Frame \(name)")
    window.setFrameAutosaveName(name)
    guard let saved = saved else { window.center(); return }
    if let frame = frame(saved), visible(frame, on: NSScreen.screens.map(\.visibleFrame)) {
      window.setFrame(frame, display: false)
    }
  }

  // The saved string starts with the frame: "x y width height" and then the
  // frame of the screen it was saved on.
  static func frame(_ saved: String) -> NSRect? {
    let values = saved.split(separator: " ").prefix(4).compactMap { Double($0) }
    guard values.count == 4, values[2] > 0, values[3] > 0 else { return nil }
    return NSRect(x: values[0], y: values[1], width: values[2], height: values[3])
  }

  // Enough of the window to grab and move it is on one screen.
  static func visible(_ frame: NSRect, on screens: [NSRect]) -> Bool {
    screens.contains { screen in
      let shown = screen.intersection(frame)
      return shown.width >= 100 && shown.height >= 100
    }
  }
}
