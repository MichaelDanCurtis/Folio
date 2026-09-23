import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

const historyMenuChannel = MethodChannel('folio/history-menu');

/// The focused field owns Undo, including an empty or composing draft. Native
/// menus observe its availability without falling through to workspace history.
class DraftHistoryTarget {
  final Object owner;
  final bool canUndo, canRedo;
  final VoidCallback undo, redo;
  const DraftHistoryTarget({
    required this.owner,
    required this.canUndo,
    required this.canRedo,
    required this.undo,
    required this.redo,
  });
}

final focusedDraftHistory = ValueNotifier<DraftHistoryTarget?>(null);
