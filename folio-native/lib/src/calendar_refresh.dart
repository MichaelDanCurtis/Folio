import 'dart:async';

typedef CalendarStamp = (int year, int month, int day, int offsetSeconds);
CalendarStamp localCalendarStamp() {
  final now = DateTime.now();
  return (now.year, now.month, now.day, now.timeZoneOffset.inSeconds);
}

/// Checks date and UTC-offset changes, including wake/travel and clock rollback.
/// The callback refreshes derived calculations without editing the workspace.
class CalendarRefresh {
  final CalendarStamp Function() clock;
  final Future<void> Function() onChange;
  final void Function(Object)? onError;
  late CalendarStamp _seen;
  Timer? _timer;
  Future<void>? _pending;
  bool _disposed = false;
  String? _lastError;
  CalendarRefresh({
    required this.onChange,
    this.onError,
    this.clock = localCalendarStamp,
  }) {
    _seen = clock();
  }
  void start() {
    if (_disposed) return;
    _timer ??= Timer.periodic(const Duration(seconds: 30), (_) {
      unawaited(
        check().catchError((Object e) {
          if (!_disposed && _lastError != e.toString()) onError?.call(e);
          _lastError = e.toString();
        }),
      );
    });
  }

  Future<void> check() {
    if (_disposed) return Future.value();
    if (_pending != null) return _pending!;
    final observed = clock();
    if (observed == _seen) return Future.value();
    final pending = Future.sync(onChange)
        .then((_) {
          if (!_disposed) _seen = observed;
          _lastError = null;
        })
        .whenComplete(() => _pending = null);
    _pending = pending;
    return pending;
  }

  void dispose() {
    _disposed = true;
    _timer?.cancel();
  }
}
