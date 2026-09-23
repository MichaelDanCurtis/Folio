import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

class InsertDocumentLineBreak extends Intent {
  const InsertDocumentLineBreak();
}

const documentLineBreakShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.enter, shift: true):
      InsertDocumentLineBreak(),
  SingleActivator(LogicalKeyboardKey.numpadEnter, shift: true):
      InsertDocumentLineBreak(),
};

void insertDocumentLineBreak(q.QuillController controller) {
  final selection = controller.selection;
  if (controller.readOnly ||
      !selection.isValid ||
      selection.end >= controller.document.length) {
    return;
  }
  // An opaque block has no editable inline position. Do not attach a break
  // to its embed and lose it when the preserved block is decoded.
  final line = controller.document.querySegmentLeafNode(selection.start).line;
  if (line == null ||
      line.children.any(
        (node) => node is q.Embed && node.value.type == 'folioNode',
      )) {
    return;
  }
  controller.replaceText(
    selection.start,
    selection.end - selection.start,
    Delta()..insert(
      {
        q.Embeddable.lineBreakType: jsonEncode({'type': 'hardBreak'}),
      },
      Map.fromEntries(
        controller
            .getSelectionStyle()
            .attributes
            .entries
            .where((e) => e.value.isInline)
            .map((e) => MapEntry(e.key, e.value.value)),
      ),
    ),
    TextSelection.collapsed(offset: selection.start + 1),
  );
}

Map<Type, Action<Intent>> documentLineBreakActions(
  q.QuillController controller,
) => {
  InsertDocumentLineBreak: CallbackAction<InsertDocumentLineBreak>(
    onInvoke: (_) {
      insertDocumentLineBreak(controller);
      return null;
    },
  ),
};
