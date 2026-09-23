import 'package:flutter/services.dart';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

/// Preserve the current reading/editing position across a committed refresh.
/// Formatting and embedded-object property changes do not move text offsets.
/// Text edits transform both endpoints, retaining reversed selections as well.
TextSelection selectionAfterDocumentRefresh(
  q.Document before,
  q.Document after,
  TextSelection selection, {
  Delta? change,
}) {
  if (!selection.isValid) return const TextSelection.collapsed(offset: 0);
  final oldText = before.toPlainText(), newText = after.toPlainText();
  change ??= oldText == newText ? null : before.toDelta().diff(after.toDelta());
  int position(int offset) {
    final bounded = offset.clamp(0, before.length - 1);
    final mapped = (change?.transformPosition(bounded) ?? bounded).clamp(
      0,
      after.length - 1,
    );
    // Offsets are UTF-16. A shortened/replaced run must not leave a caret
    // between the two code units of an astral character.
    if (mapped > 0 && mapped < newText.length) {
      final left = newText.codeUnitAt(mapped - 1);
      final right = newText.codeUnitAt(mapped);
      if (left >= 0xd800 &&
          left <= 0xdbff &&
          right >= 0xdc00 &&
          right <= 0xdfff) {
        return mapped - 1;
      }
    }
    return mapped;
  }

  return selection.copyWith(
    baseOffset: position(selection.baseOffset),
    extentOffset: position(selection.extentOffset),
  );
}
