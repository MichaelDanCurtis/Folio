#!/usr/bin/env python3
"""Exercise the actual window-close method against channel results, without UI."""
from pathlib import Path
import subprocess, tempfile
root = Path(__file__).resolve().parents[1]
source = (root / 'macos/Runner/MainFlutterWindow.swift').read_text()
method = source[source.index('  func prepareClose('):source.index('  override func close()')]
checks = '''
final class FlutterError { var message: String? }
final class Channel {
  var value: Any? = false
  var calls = 0
  func invokeMethod(_ name: String, arguments: Any?, result: (Any?) -> Void) {
    precondition(name == "prepareClose")
    calls += 1
    result(value)
  }
}
final class WindowHarness {
  var closing = false
  var lifecycle: Channel? = Channel()
METHOD
}
let window = WindowHarness()
for value: Any? in [false, nil, "true", NSNull()] {
  window.lifecycle!.value = value
  window.prepareClose { precondition(!$0, "Only explicit approval may close") }
}
window.lifecycle!.value = true
window.prepareClose { precondition($0) }
precondition(window.lifecycle!.calls == 5)
window.closing = true
window.lifecycle!.value = false
window.prepareClose { precondition($0, "Completed window close must not prompt again") }
precondition(window.lifecycle!.calls == 5)
print("Production window close honors Cancel, rejects absent/invalid approval, and avoids duplicate close prompts.")
'''.replace('METHOD', method)
with tempfile.TemporaryDirectory(prefix='folio-close-check-') as temporary:
    script = Path(temporary) / 'main.swift'
    script.write_text('import Cocoa\n' + checks)
    subprocess.run(['xcrun', '--sdk', 'macosx', 'swift', str(script)], check=True)
