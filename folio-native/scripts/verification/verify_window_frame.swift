import Cocoa

@main
struct VerifyWindowFrame {
  static func main() {
    _ = NSApplication.shared
    let name = "FolioFrameCheck-\(UUID().uuidString)"
    let key = "NSWindow Frame \(name)"
    defer { UserDefaults.standard.removeObject(forKey: key) }
    func window() -> NSWindow {
      let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400),
                            styleMask: [.titled, .resizable, .fullSizeContentView],
                            backing: .buffered, defer: true)
      window.minSize = NSSize(width: 300, height: 200)
      return window
    }

    precondition(FolioWindowFrame.frame("200 103 1440 864 0 0 1728 1084 ") == NSRect(x: 200, y: 103, width: 1440, height: 864))
    precondition(FolioWindowFrame.frame("200 103 0 864") == nil && FolioWindowFrame.frame("") == nil)
    let screen = NSRect(x: 0, y: 0, width: 1728, height: 1084)
    precondition(FolioWindowFrame.visible(NSRect(x: 1600, y: 0, width: 600, height: 400), on: [screen]))
    precondition(!FolioWindowFrame.visible(NSRect(x: 1650, y: 0, width: 600, height: 400), on: [screen]))
    precondition(!FolioWindowFrame.visible(NSRect(x: 5000, y: 0, width: 600, height: 400), on: [screen]))

    let first = window()
    FolioWindowFrame.restore(first, name: name)
    precondition(first.frame.origin != .zero, "a first launch centres the window")

    // Every connected display, including ones without focus (not NSScreen.main):
    // AppKit's restore would move these onto the main screen.
    for (index, screen) in NSScreen.screens.enumerated() {
      let area = screen.visibleFrame
      let moved = NSRect(x: area.minX + 37, y: area.minY + 23, width: 600, height: 400)
      let before = window()
      FolioWindowFrame.restore(before, name: name)
      before.setFrame(moved, display: false)
      before.saveFrame(usingName: name)
      let after = window()
      FolioWindowFrame.restore(after, name: name)
      precondition(after.frame == moved, "screen \(index): relaunch restores \(moved), got \(after.frame)")
      let stored = UserDefaults.standard.string(forKey: key) ?? ""
      precondition(FolioWindowFrame.frame(stored) == moved, "screen \(index): the saved frame is kept, got \(stored)")
    }

    // A display that has gone: AppKit brings the window back onto a screen.
    UserDefaults.standard.set("90000 90000 600 400 90000 90000 1728 1084 ", forKey: key)
    let lost = window()
    FolioWindowFrame.restore(lost, name: name)
    precondition(FolioWindowFrame.visible(lost.frame, on: NSScreen.screens.map(\.visibleFrame)),
                 "a frame from a missing display comes back on screen, got \(lost.frame)")
    print("window frame: first launch centred; restored exactly on \(NSScreen.screens.count) screen(s); missing display brought back")
  }
}
