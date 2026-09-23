import 'package:flutter/services.dart';
import 'package:flutter/foundation.dart';

/// Modifier state belongs to the key's arrival, not a later save completion.
class SheetEntryKey {
  final KeyEvent event;
  final bool isAltPressed, isControlPressed, isMetaPressed, isShiftPressed;

  SheetEntryKey(this.event)
    : isAltPressed = HardwareKeyboard.instance.isAltPressed,
      isControlPressed = HardwareKeyboard.instance.isControlPressed,
      isMetaPressed = HardwareKeyboard.instance.isMetaPressed,
      isShiftPressed = HardwareKeyboard.instance.isShiftPressed;

  bool get down => event is KeyDownEvent || event is KeyRepeatEvent;
  bool get completion => const [
    LogicalKeyboardKey.enter,
    LogicalKeyboardKey.numpadEnter,
    LogicalKeyboardKey.tab,
    LogicalKeyboardKey.escape,
  ].contains(event.logicalKey);

  bool get text =>
      !isMetaPressed &&
      !isControlPressed &&
      event.character != null &&
      event.character!.isNotEmpty &&
      !event.character!.runes.any((r) => r < 32 || r == 127);

  bool get caretArrow => const [
    LogicalKeyboardKey.arrowLeft,
    LogicalKeyboardKey.arrowRight,
    LogicalKeyboardKey.arrowUp,
    LogicalKeyboardKey.arrowDown,
  ].contains(event.logicalKey);

  bool get canQueue {
    if (!down || isMetaPressed) return false;
    if (text) return true;
    if (isAltPressed &&
        !isShiftPressed &&
        (!isControlPressed || defaultTargetPlatform == TargetPlatform.macOS) &&
        (event.logicalKey == LogicalKeyboardKey.enter ||
            event.logicalKey == LogicalKeyboardKey.numpadEnter)) {
      return true;
    }
    if (isAltPressed || isControlPressed) return false;
    return completion ||
        caretArrow ||
        const [
          LogicalKeyboardKey.backspace,
          LogicalKeyboardKey.delete,
          LogicalKeyboardKey.f2,
        ].contains(event.logicalKey);
  }
}
