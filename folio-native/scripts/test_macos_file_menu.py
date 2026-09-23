#!/usr/bin/env python3
"""Exercise the production AppKit menu in memory; no desktop input is posted."""
from pathlib import Path
import subprocess, tempfile
root = Path(__file__).resolve().parents[1]
source = (root/'macos/Runner/MainFlutterWindow.swift').read_text()
start = source.index('final class FolioFileMenu:')
end = source.index('class FolioFlutterViewController:', start)
checks = r'''
final class TestWindow: NSWindow {
  var simulatedVisible = true
  var simulatedSheet: NSWindow?
  override var isVisible: Bool { simulatedVisible }
  override var attachedSheet: NSWindow? { simulatedSheet }
}
_ = NSApplication.shared
NSApp.mainMenu = NSMenu(title: "Main")
NSApp.mainMenu!.addItem(NSMenuItem(title: "App", action: nil, keyEquivalent: ""))
NSApp.mainMenu!.addItem(NSMenuItem(title: "Edit", action: nil, keyEquivalent: ""))
let window = TestWindow(contentRect: .zero, styleMask: [], backing: .buffered, defer: true)
let file = FolioFileMenu(window: window)
let commands = file.menu.items.filter { !$0.isSeparatorItem }
func item(_ name: String) -> NSMenuItem { commands.first { ($0.representedObject as? String) == name }! }
let openIndex = file.menu.items.firstIndex { ($0.representedObject as? String) == "open" }!
var calls: [String] = []
var complete: (() -> Void)?
file.onCommand = { command, done in calls.append(command); complete = done }
precondition(commands.allSatisfy { !file.validateMenuItem($0) })
file.update(available: true, canReplace: true, spreadsheet: false)
precondition(NSApp.mainMenu!.items.map(\.title) == ["App", "File", "Edit"])
precondition(commands.allSatisfy { file.validateMenuItem($0) })
precondition(commands.map(\.keyEquivalent) == ["n", "", "o", "", "s", "s", "", "", "e", ""])
precondition(item("export").keyEquivalentModifierMask == [.command, .shift])
precondition(item("open").keyEquivalentModifierMask == .command)
file.menu.performActionForItem(at: openIndex)
precondition(calls == ["open"])
precondition(commands.allSatisfy { !file.validateMenuItem($0) })
file.menu.performActionForItem(at: 0)
precondition(calls == ["open"], "Duplicate native action was not suppressed")
complete!()
precondition(commands.allSatisfy { file.validateMenuItem($0) })
file.update(available: true, canReplace: false, spreadsheet: true)
precondition(["newWorkspace", "fromTemplate", "open", "recent"].allSatisfy { !file.validateMenuItem(item($0)) })
precondition(["save", "saveAs", "saveCopy", "saveTemplate", "export"].allSatisfy { file.validateMenuItem(item($0)) })
precondition(item("export").title == "Export Excel…")
precondition(NSApp.mainMenu!.numberOfItems == 3, "Rebuild duplicated File")
window.simulatedSheet = NSWindow(contentRect: .zero, styleMask: [], backing: .buffered, defer: true)
precondition(commands.allSatisfy { !file.validateMenuItem($0) })
window.simulatedSheet = nil
window.simulatedVisible = false
precondition(commands.allSatisfy { !file.validateMenuItem($0) })
window.simulatedVisible = true
file.update(available: false, canReplace: true, spreadsheet: false)
precondition(commands.allSatisfy { !file.validateMenuItem($0) })
precondition(item("export").title == "Export Word…")
print("AppKit File menu: order, modifiers, dispatch, duplicates, dialogs, busy and view state passed (no desktop events).")
'''
with tempfile.TemporaryDirectory(prefix='folio-file-menu-check-') as temporary:
    script = Path(temporary)/'main.swift'
    script.write_text('import Cocoa\n'+source[start:end]+checks)
    subprocess.run(['xcrun','--sdk','macosx','swift',str(script)],check=True)
