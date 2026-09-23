// Compile with macos/Runner/FolioActivationRecovery.swift. This exercises
// notification ordering and cancellation without launching or controlling UI.
import Cocoa

@main
struct VerifyActivationRecovery {
  static func main() {
    let center = NotificationCenter()
    let window = NSObject()
    let workspaceCenter = NotificationCenter()
    var state = FolioActivationRecovery.State(
      active: true, hidden: false, visibleWindow: true, minimized: false, keyWindow: true)
    var messages: [String] = []
    var recovery: FolioActivationRecovery? = FolioActivationRecovery(
      window: window, center: center, workspaceCenter: workspaceCenter, state: { state }, resume: { messages.append("resumed") })

    func activate() {
      center.post(name: NSApplication.didBecomeActiveNotification, object: nil)
    }
    func drain() {
      RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.02))
    }

    // The engine's synchronous stale-hidden message must precede recovery.
    activate()
    precondition(messages.isEmpty)
    messages.append("engine-hidden")
    drain()
    precondition(messages == ["engine-hidden", "resumed"])

    // A window can close after activation but before the queued callback.
    activate()
    state = .init(active: true, hidden: false, visibleWindow: false, minimized: false, keyWindow: true)
    drain()
    precondition(messages.count == 2)

    for blocked in [
      FolioActivationRecovery.State(active: false, hidden: false, visibleWindow: true, minimized: false, keyWindow: true),
      .init(active: true, hidden: true, visibleWindow: true, minimized: false, keyWindow: true),
      .init(active: true, hidden: false, visibleWindow: true, minimized: true, keyWindow: true),
      .init(active: true, hidden: false, visibleWindow: true, minimized: false, keyWindow: false),
    ] {
      state = blocked
      activate()
      drain()
      precondition(messages.count == 2)
    }

    state = .init(active: true, hidden: false, visibleWindow: true, minimized: false, keyWindow: true)
    activate()
    drain()
    precondition(messages.count == 3)
    // Restoring this window while the app stays active also repairs hidden.
    center.post(name: NSWindow.didDeminiaturizeNotification, object: window)
    drain()
    precondition(messages.count == 4)
    center.post(name: NSWindow.didBecomeKeyNotification, object: window)
    drain()
    precondition(messages.count == 5)
    // Other windows must not resume this engine.
    center.post(name: NSWindow.didBecomeKeyNotification, object: NSObject())
    drain()
    precondition(messages.count == 5)
    // A foreground display transfer can incorrectly mark the app occluded.
    // Coalesce its application/window notifications into one recovery message.
    center.post(name: NSApplication.didChangeOcclusionStateNotification, object: nil)
    center.post(name: NSWindow.didChangeOcclusionStateNotification, object: window)
    center.post(name: NSWindow.didChangeScreenNotification, object: window)
    drain()
    precondition(messages.count == 6)

    // Inactive/non-key windows stay hidden even when their occlusion changes.
    state = .init(active: false, hidden: false, visibleWindow: true, minimized: false, keyWindow: false)
    center.post(name: NSApplication.didChangeOcclusionStateNotification, object: nil)
    drain()
    precondition(messages.count == 6)
    state = .init(active: true, hidden: false, visibleWindow: true, minimized: false, keyWindow: true)
    // Sleep and session suspension are independent guards, including a
    // suspension arriving after recovery was queued but before it executes.
    activate()
    workspaceCenter.post(name: NSWorkspace.screensDidSleepNotification, object: nil)
    workspaceCenter.post(name: NSWorkspace.sessionDidResignActiveNotification, object: nil)
    drain()
    precondition(messages.count == 6)
    workspaceCenter.post(name: NSWorkspace.screensDidWakeNotification, object: nil)
    drain()
    precondition(messages.count == 6)
    workspaceCenter.post(name: NSWorkspace.sessionDidBecomeActiveNotification, object: nil)
    drain()
    precondition(messages.count == 7)

    workspaceCenter.post(name: NSWorkspace.screensDidSleepNotification, object: nil)
    workspaceCenter.post(name: NSWorkspace.sessionDidResignActiveNotification, object: nil)
    workspaceCenter.post(name: NSWorkspace.sessionDidBecomeActiveNotification, object: nil)
    drain()
    precondition(messages.count == 7)
    workspaceCenter.post(name: NSWorkspace.screensDidWakeNotification, object: nil)
    drain()
    precondition(messages.count == 8)

    activate()
    weak let released = recovery
    recovery = nil
    precondition(released == nil)
    drain()
    activate()
    drain()
    precondition(messages.count == 8)
    print("Activation, display transfer, suspension guards, coalescing and teardown passed.")
  }
}
