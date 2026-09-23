#!/usr/bin/env python3
"""Test the production AppKit event adapter without launching the workspace app.

Only constructs in-memory NSEvents; no events are posted to the desktop.
Requires the Xcode command-line tools on macOS.
"""
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
source = (root / "macos/Runner/MainFlutterWindow.swift").read_text()
start = source.index("enum FolioKeyboardEvent {")
end = source.index("class FolioFlutterViewController:", start)
production_adapter = source[start:end]
checks = r'''
func event(_ type: NSEvent.EventType, _ flags: NSEvent.ModifierFlags,
           _ key: UInt16 = 48) -> NSEvent {
  return NSEvent.keyEvent(with: type, location: NSPoint(x: 4, y: 7),
    modifierFlags: flags, timestamp: 123.25, windowNumber: 0, context: nil,
    characters: type == .flagsChanged ? "" : "\t",
    charactersIgnoringModifiers: type == .flagsChanged ? "" : "\t",
    isARepeat: false, keyCode: key)!
}
var cases = 0
for (type, flags, text, expected): (NSEvent.EventType, NSEvent.ModifierFlags, String, Bool) in [
  (.keyDown, .command, "q", true),
  (.keyDown, [.command, .capsLock], "Q", true),
  (.keyUp, .command, "q", false),
  (.keyDown, [], "q", false),
  (.keyDown, [.command, .option], "q", false),
  (.keyDown, [.command, .control], "q", false),
  (.keyDown, [.command, .shift], "q", false),
  (.keyDown, .command, "z", false),
] {
  let input = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: flags,
    timestamp: 1, windowNumber: 0, context: nil, characters: text,
    charactersIgnoringModifiers: text, isARepeat: false, keyCode: 12)!
  precondition(FolioKeyboardEvent.isQuitShortcut(input) == expected)
  cases += 1
}
let responderCases: [(FolioKeyboardEvent.ResponderRole, Bool, Bool, Bool, String)] = [
  (.window, false, false, true, "document keys after dialog detachment"),
  (.missing, false, false, true, "missing first responder"),
  (.other, false, false, false, "active text field keeps its input route"),
  (.controller, true, false, false, "already routed controller"),
  (.other, true, false, true, "grid reclaims stale text responder"),
  (.window, false, true, false, "document native Save panel"),
  (.other, true, true, false, "grid native Save panel"),
  (.missing, true, true, false, "native panel with no main responder"),
  (.window, true, true, false, "native panel with window responder"),
  (.controller, false, false, false, "document controller needs no change"),
]
for (role, grid, sheet, expected, scenario) in responderCases {
  precondition(FolioKeyboardEvent.shouldRestoreResponder(role, gridFocused: grid, hasAttachedSheet: sheet) == expected, scenario)
  cases += 1
}
let modifiers: [(NSEvent.ModifierFlags, UInt, UInt)] = [
  (.shift, 0x2, 0x4), (.control, 0x1, 0x2000),
  (.option, 0x20, 0x40), (.command, 0x8, 0x10)
]
for (aggregate, left, right) in modifiers {
  for type in [NSEvent.EventType.keyDown, .keyUp, .flagsChanged] {
    let original = event(type, aggregate)
    let normalized = FolioKeyboardEvent.normalize(original)
    precondition(normalized.modifierFlags.rawValue == aggregate.rawValue | left)
    precondition(normalized.type == type)
    precondition(normalized.keyCode == original.keyCode)
    precondition(normalized.timestamp == original.timestamp)
    precondition(normalized.locationInWindow == original.locationInWindow)
    if type != .flagsChanged {
      precondition(normalized.characters == original.characters)
      precondition(normalized.charactersIgnoringModifiers == original.charactersIgnoringModifiers)
      precondition(normalized.isARepeat == original.isARepeat)
    }
    cases += 1
  }
  for sides in [left, right, left | right] {
    let flags = NSEvent.ModifierFlags(rawValue: aggregate.rawValue | sides)
    let original = event(.keyDown, flags)
    precondition(FolioKeyboardEvent.normalize(original) === original)
    cases += 1
  }
}
let combined: NSEvent.ModifierFlags = [.command, .shift, .capsLock, .function]
precondition(FolioKeyboardEvent.modifierFlags(combined).rawValue == combined.rawValue | 0xA)
cases += 1
for flags: NSEvent.ModifierFlags in [[], .capsLock, .function, .numericPad] {
  let original = event(.flagsChanged, flags, 0)
  precondition(FolioKeyboardEvent.normalize(original) === original)
  cases += 1
}
print("\(cases) AppKit keyboard adapter checks passed (in-memory events only).")
'''
with tempfile.TemporaryDirectory(prefix="folio-keyboard-check-") as temporary:
    script = Path(temporary) / "main.swift"
    script.write_text("import Cocoa\n" + production_adapter + checks)
    subprocess.run(["xcrun", "--sdk", "macosx", "swift", str(script)], check=True)
