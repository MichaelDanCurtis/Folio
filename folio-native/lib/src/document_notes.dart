// Footnotes and endnotes (W11.01): the reference mark in the editor, its
// number, and the dialog that writes the note. The engine keeps each note's
// text on its reference (`noteRef`) and writes it into the Word package.
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

const noteKinds = {'footnote': 'Footnote', 'endnote': 'Endnote'};

String _roman(int n) {
  const values = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
  const symbols = [
    'm',
    'cm',
    'd',
    'cd',
    'c',
    'xc',
    'l',
    'xl',
    'x',
    'ix',
    'v',
    'iv',
    'i',
  ];
  final out = StringBuffer();
  for (var i = 0; i < values.length; i++) {
    while (n >= values[i]) {
      out.write(symbols[i]);
      n -= values[i];
    }
  }
  return out.toString();
}

/// The mark of the [index]th note (from 1) of a kind: footnotes 1, 2, 3,
/// endnotes i, ii, iii, as in Word.
String noteMark(String kind, int index) =>
    kind == 'endnote' ? _roman(index) : '$index';

/// The mark of each note reference in a document, in reading order.
Map<Map, String> documentNoteMarks(Map document) {
  final marks = Map<Map, String>.identity();
  final counts = <String, int>{};
  void walk(Object? n) {
    if (n is Map) {
      if (n['type'] == 'noteRef') {
        final kind = n['attrs']?['kind'] as String? ?? 'footnote';
        counts[kind] = (counts[kind] ?? 0) + 1;
        marks[n] = noteMark(kind, counts[kind]!);
      }
      (n['content'] as List?)?.forEach(walk);
    }
  }

  walk(document);
  return marks;
}

Map<String, dynamic> newNote(String kind, String text) => {
  'type': 'noteRef',
  'attrs': {'kind': kind, 'id': null, 'text': text},
};

/// Asks for a note's text. Null when cancelled.
Future<String?> editNoteText(
  BuildContext context, {
  required String kind,
  String text = '',
  String? mark,
}) => showDialog<String>(
  context: context,
  builder: (context) => _NoteDialog(kind: kind, text: text, mark: mark),
);

/// Owns its text controller for the dialog's whole life, closing
/// animation included.
class _NoteDialog extends StatefulWidget {
  final String kind, text;
  final String? mark;
  const _NoteDialog({required this.kind, required this.text, this.mark});
  @override
  State<_NoteDialog> createState() => _NoteDialogState();
}

class _NoteDialogState extends State<_NoteDialog> {
  late final controller = TextEditingController(text: widget.text);

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(
      widget.mark == null
          ? 'New ${noteKinds[widget.kind]!.toLowerCase()}'
          : '${noteKinds[widget.kind]} ${widget.mark}',
    ),
    content: SizedBox(
      width: 460,
      child: TextField(
        key: const Key('note-text'),
        controller: controller,
        autofocus: true,
        minLines: 3,
        maxLines: 8,
        decoration: const InputDecoration(
          border: OutlineInputBorder(),
          hintText: 'Note text',
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('note-save'),
        onPressed: () => Navigator.pop(context, controller.text.trimRight()),
        child: const Text('OK'),
      ),
    ],
  );
}

/// Inserts a footnote or endnote at the selection, after asking for its text.
Future<void> insertNote(
  BuildContext context,
  q.QuillController controller,
  String kind,
) async {
  if (controller.readOnly) return;
  final selection = controller.selection;
  if (!selection.isValid) return;
  final text = await editNoteText(context, kind: kind);
  if (text == null) return;
  controller.replaceText(
    selection.end,
    0,
    q.Embeddable('folioNote', jsonEncode(newNote(kind, text))),
    TextSelection.collapsed(offset: selection.end + 1),
  );
}

/// The number of the note at [offset] among the notes of its kind before it.
String liveNoteMark(q.QuillController controller, int offset, String kind) {
  var count = 0, position = 0;
  for (final op in controller.document.toDelta().toList()) {
    if (position > offset) break;
    final insert = op.data;
    if (insert is Map && insert['folioNote'] is String) {
      final node = jsonDecode(insert['folioNote'] as String) as Map;
      if (node['attrs']?['kind'] == kind) count++;
    }
    position += op.length!;
  }
  return noteMark(kind, count == 0 ? 1 : count);
}

class NoteEmbedBuilder extends q.EmbedBuilder {
  const NoteEmbedBuilder();
  @override
  String get key => 'folioNote';
  @override
  bool get expanded => false;
  @override
  String toPlainText(q.Embed node) => '';

  @override
  Widget build(BuildContext context, q.EmbedContext embedContext) {
    final node = Map<String, dynamic>.from(
      jsonDecode(embedContext.node.value.data) as Map,
    );
    final attrs = Map<String, dynamic>.from(node['attrs'] as Map);
    final kind = attrs['kind'] as String? ?? 'footnote';
    final offset = embedContext.node.documentOffset;
    final mark = liveNoteMark(embedContext.controller, offset, kind);
    final text = attrs['text'] as String? ?? '';
    final style = embedContext.textStyle;
    return Semantics(
      button: true,
      label: '${noteKinds[kind]} $mark: $text',
      child: ExcludeSemantics(
        child: Tooltip(
          message: text.isEmpty ? '${noteKinds[kind]} $mark' : text,
          child: MouseRegion(
            cursor: SystemMouseCursors.click,
            child: GestureDetector(
              key: Key('note-$kind-$mark'),
              onTap: embedContext.readOnly
                  ? null
                  : () async {
                      final edited = await editNoteText(
                        context,
                        kind: kind,
                        text: text,
                        mark: mark,
                      );
                      if (edited == null || edited == text) return;
                      attrs['text'] = edited;
                      node['attrs'] = attrs;
                      embedContext.controller.replaceText(
                        offset,
                        1,
                        q.Embeddable('folioNote', jsonEncode(node)),
                        null,
                      );
                    },
              child: Transform.translate(
                offset: Offset(0, -(style.fontSize ?? 14) * 0.35),
                child: Text(
                  mark,
                  style: style.copyWith(
                    fontSize: (style.fontSize ?? 14) * 0.7,
                    color: const Color(0xff185abd),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
