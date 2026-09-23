import Cocoa

/// Foreground-window recovery for Flutter 3.44's stale macOS occlusion state.
/// See https://github.com/flutter/flutter/pull/188772. Remove after upgrading
/// to an engine containing that fix and repeating the native lifecycle checks.
final class FolioActivationRecovery {
  struct State {
    let active: Bool
    let hidden: Bool
    let visibleWindow: Bool
    let minimized: Bool
    let keyWindow: Bool
  }

  private let center: NotificationCenter
  private let state: () -> State
  private let resume: () -> Void
  private var observers: [(NotificationCenter, NSObjectProtocol)] = []
  private var recoveryQueued = false
  private var sessionInactive = false
  private var displaysAsleep = false

  init(window: AnyObject, center: NotificationCenter = .default,
       workspaceCenter: NotificationCenter = NSWorkspace.shared.notificationCenter,
       state: @escaping () -> State, resume: @escaping () -> Void) {
    self.center = center
    self.state = state
    self.resume = resume
    let events: [(Notification.Name, AnyObject?)] = [
      (NSApplication.didBecomeActiveNotification, nil),
      // Restoring or selecting a window need not reactivate an already active
      // app, and its occlusion state can be stale in the same way.
      (NSWindow.didDeminiaturizeNotification, window),
      (NSWindow.didBecomeKeyNotification, window),
      (NSWindow.didChangeScreenNotification, window),
      // On display transfers AppKit can report occluded while the window is
      // still active, ordered in and key. Activation alone does not fire then.
      (NSWindow.didChangeOcclusionStateNotification, window),
      (NSApplication.didChangeOcclusionStateNotification, nil),
    ]
    for (name, object) in events {
      let observer = center.addObserver(forName: name, object: object, queue: .main) { [weak self] _ in
        self?.queueRecovery()
      }
      observers.append((center, observer))
    }
    let suspensionEvents: [(Notification.Name, Bool, Bool)] = [
      (NSWorkspace.sessionDidResignActiveNotification, true, true),
      (NSWorkspace.sessionDidBecomeActiveNotification, true, false),
      (NSWorkspace.screensDidSleepNotification, false, true),
      (NSWorkspace.screensDidWakeNotification, false, false),
    ]
    for (name, session, suspended) in suspensionEvents {
      let observer = workspaceCenter.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
        guard let self = self else { return }
        if session { self.sessionInactive = suspended } else { self.displaysAsleep = suspended }
        if !suspended { self.queueRecovery() }
      }
      observers.append((workspaceCenter, observer))
    }
  }

  private func queueRecovery() {
    guard !recoveryQueued else { return }
    recoveryQueued = true
    // Run after Flutter forwards the native notifications, and re-read state
    // in case activation was followed by minimizing, hiding or locking.
    DispatchQueue.main.async { [weak self] in
      guard let self = self else { return }
      self.recoveryQueued = false
      self.resumeVisibleWindow()
    }
  }

  private func resumeVisibleWindow() {
    let current = state()
    guard current.active, current.keyWindow, !current.hidden, current.visibleWindow,
          !current.minimized, !sessionInactive, !displaysAsleep else { return }
    resume()
  }

  deinit {
    for (center, observer) in observers { center.removeObserver(observer) }
  }
}
