import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter/services.dart';
import 'document_codec.dart';

/// Positions are Quill/Flutter UTF-16 offsets, including embedded objects.
/// A revision and exact original text make stale selection edits fail closed.
class DocumentAnchor {
  final int start, end, revision;
  final String workspaceId, text;
  final List<dynamic> delta;
  DocumentAnchor._(
    this.start,
    this.end,
    this.revision,
    this.workspaceId,
    this.text,
    this.delta,
  );
  factory DocumentAnchor.capture(
    q.QuillController editor,
    int revision,
    String workspaceId,
  ) {
    final plain = editor.document.toPlainText();
    var start = editor.selection.start.clamp(0, plain.length - 1),
        end = editor.selection.end.clamp(0, plain.length - 1);
    if (start == end) {
      start = start == 0 ? 0 : plain.lastIndexOf('\n', start - 1) + 1;
      end = plain.indexOf('\n', end);
      if (end < 0) end = plain.length - 1;
    }
    return DocumentAnchor._(
      start,
      end,
      revision,
      workspaceId,
      plain.substring(start, end),
      editor.document.toDelta().toJson(),
    );
  }
  Map<String, dynamic> toJson() => {
    'kind': 'document',
    'start': start,
    'end': end,
    'text': text,
    'baseRevision': revision,
    'workspaceId': workspaceId,
  };
  Map<String, dynamic> replace(
    q.QuillController editor, {
    required int currentRevision,
    required String currentWorkspaceId,
    required String expectedText,
    required String replacement,
  }) {
    if (currentRevision != revision ||
        currentWorkspaceId != workspaceId ||
        expectedText != text ||
        editor.document.toPlainText().substring(start, end) != text) {
      throw StateError(
        'The selected passage changed. Select it again and request an updated proposal.',
      );
    }
    if (text.contains('\uFFFC')) {
      throw StateError('Select text rather than an embedded table or chart.');
    }
    if (replacement.length > 30000) {
      throw StateError('Replacement exceeds 30,000 characters.');
    }
    final copy = q.QuillController(
      document: q.Document.fromDelta(editor.document.toDelta()),
      selection: TextSelection(baseOffset: start, extentOffset: end),
    );
    try {
      copy.replaceText(
        start,
        end - start,
        replacement,
        TextSelection.collapsed(offset: start + replacement.length),
      );
      return deltaToDocument(copy.document.toDelta());
    } finally {
      copy.dispose();
    }
  }
}
