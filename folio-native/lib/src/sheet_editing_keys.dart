import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'sheet_entry_key.dart';

/// Spreadsheet commit/navigation keys around a standard multiline text field.
/// All other keys retain EditableText's caret, clipboard and local-history path.
class SheetEditingKeys extends StatelessWidget {
  final TextEditingController controller;
  final FocusNode focus;
  final void Function(int rows, int columns) onCommit;
  final VoidCallback onCancel;
  final KeyEventResult Function(KeyEvent)? intercept;
  final Widget child;

  const SheetEditingKeys({
    super.key,
    required this.controller,
    required this.focus,
    required this.onCommit,
    required this.onCancel,
    this.intercept,
    required this.child,
  });

  KeyEventResult _key(FocusNode node, KeyEvent event) {
    final result = intercept?.call(event) ?? KeyEventResult.ignored;
    if (result != KeyEventResult.ignored) return result;
    return handle(
      event,
      controller: controller,
      focus: focus,
      onCommit: onCommit,
      onCancel: onCancel,
    );
  }

  /// Shared by the attached field and its grid-to-field focus handoff.
  static KeyEventResult handle(
    KeyEvent event, {
    required TextEditingController controller,
    required FocusNode focus,
    required void Function(int rows, int columns) onCommit,
    required VoidCallback onCancel,
    bool beforeAttachment = false,
    SheetEntryKey? input,
  }) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final key = event.logicalKey;
    final enter =
        key == LogicalKeyboardKey.enter ||
        key == LogicalKeyboardKey.numpadEnter;
    if (!enter &&
        key != LogicalKeyboardKey.tab &&
        key != LogicalKeyboardKey.escape) {
      return KeyEventResult.ignored;
    }
    // Let the input method own candidate acceptance/cancellation. In particular,
    // Enter during a live composition must never save a partial cell draft.
    if (controller.value.composing.isValid &&
        !controller.value.composing.isCollapsed) {
      return KeyEventResult.skipRemainingHandlers;
    }
    final keyboard = input ?? SheetEntryKey(event);
    if (enter &&
        keyboard.isAltPressed &&
        !keyboard.isMetaPressed &&
        !keyboard.isShiftPressed &&
        (!keyboard.isControlPressed ||
            defaultTargetPlatform == TargetPlatform.macOS)) {
      final editable = focus.context
          ?.findAncestorStateOfType<EditableTextState>();
      final value = controller.value;
      final selection = value.selection;
      if (selection.isValid) {
        final updated = value.copyWith(
          text: value.text.replaceRange(selection.start, selection.end, '\n'),
          selection: TextSelection.collapsed(offset: selection.start + 1),
          composing: TextRange.empty,
        );
        if (beforeAttachment) {
          controller.value = updated;
        } else {
          editable?.userUpdateTextEditingValue(
            updated,
            SelectionChangedCause.keyboard,
          );
        }
      }
      return KeyEventResult.handled;
    }
    if (keyboard.isAltPressed ||
        keyboard.isControlPressed ||
        keyboard.isMetaPressed) {
      return KeyEventResult.ignored;
    }
    // Repeated commit keys must not become newline input while a save awaits I/O.
    if (event is KeyDownEvent) {
      if (key == LogicalKeyboardKey.escape) {
        onCancel();
      } else {
        final direction = keyboard.isShiftPressed ? -1 : 1;
        onCommit(enter ? direction : 0, enter ? 0 : direction);
      }
    }
    return KeyEventResult.handled;
  }

  @override
  Widget build(BuildContext context) => Focus(
    canRequestFocus: false,
    skipTraversal: true,
    onKeyEvent: _key,
    child: child,
  );
}
