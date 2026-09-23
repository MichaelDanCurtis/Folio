import Cocoa
import FlutterMacOS

@main
class AppDelegate: FlutterAppDelegate {
  private let termination = FolioTermination()
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool { true }
  override func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
    guard let window = MainFlutterWindow.current else { return .terminateNow }
    return termination.request(prepare: window.prepareClose) { sender.terminate(nil) }
  }
}
