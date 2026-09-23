import Cocoa

/// Keep AppKit's normal event loop running while Dart drains edits and saves.
/// A second quit request must not bypass a save that is still in progress.
final class FolioTermination {
  private var preparing = false
  private var approved = false

  func request(
    prepare: (@escaping (Bool) -> Void) -> Void,
    terminate: @escaping () -> Void
  ) -> NSApplication.TerminateReply {
    if approved { return .terminateNow }
    if preparing { return .terminateCancel }
    preparing = true
    prepare { [weak self] success in
      // Even a synchronous callback must return from applicationShouldTerminate
      // before calling terminate again, following FlutterAppDelegate's protocol.
      DispatchQueue.main.async {
        guard let self = self else { return }
        self.preparing = false
        guard success else { return }
        self.approved = true
        terminate()
      }
    }
    return .terminateCancel
  }
}
