// Change Case and Clear All Formatting (W01.13), as pure changes over the
// Quill document so each command is one Undo step.
import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart' show StringCharacters;
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_builtin_styles.dart';
import 'document_lists.dart';
import 'document_style_resolution.dart';

enum TextCase { sentence, lower, upper, capitalize, toggle }

const textCaseLabels = {
  TextCase.sentence: 'Sentence case.',
  TextCase.lower: 'lowercase',
  TextCase.upper: 'UPPERCASE',
  TextCase.capitalize: 'Capitalize Each Word',
  TextCase.toggle: 'tOGGLE cASE',
};

bool _letter(String c) => c.toLowerCase() != c.toUpperCase();
bool _wordChar(String c) => _letter(c) || RegExp(r"[0-9'’]").hasMatch(c);

/// [chars] in [mode], one output string per input character (so an ß that
/// becomes SS still maps back to its run). Sentences start at the beginning
/// and after `.`, `!` or `?` followed by white space, or a new paragraph.
List<String> caseCharacters(List<String> chars, TextCase mode) {
  final out = <String>[];
  var sentenceStart = true, afterStop = false;
  for (var i = 0; i < chars.length; i++) {
    final c = chars[i];
    final wordStart = i == 0 || !_wordChar(chars[i - 1]);
    out.add(switch (mode) {
      TextCase.lower => c.toLowerCase(),
      TextCase.upper => c.toUpperCase(),
      TextCase.toggle =>
        c == c.toUpperCase() ? c.toLowerCase() : c.toUpperCase(),
      TextCase.capitalize => wordStart ? c.toUpperCase() : c.toLowerCase(),
      TextCase.sentence =>
        sentenceStart && _letter(c) ? c.toUpperCase() : c.toLowerCase(),
    });
    if (_letter(c) || RegExp(r'[0-9]').hasMatch(c)) {
      sentenceStart = false;
      afterStop = false;
    } else if ('.!?'.contains(c)) {
      afterStop = true;
    } else if (c == '\n' || (afterStop && c.trim().isEmpty)) {
      sentenceStart = true;
    } else if (!'"”’)]'.contains(c)) {
      afterStop = false;
    }
  }
  return out;
}

String changeCase(String text, TextCase mode) =>
    caseCharacters(text.characters.toList(), mode).join();

/// The document's text with each embed as one object character, so string
/// offsets match document offsets.
String documentText(Delta delta) => delta
    .toList()
    .map((op) => op.data is String ? op.data as String : '\uFFFC')
    .join();

/// Shift+F3: UPPERCASE, then lowercase, then Capitalize Each Word.
TextCase nextCaseCycle(String text) {
  final letters = text.characters.where(_letter).join();
  if (letters.isEmpty) return TextCase.upper;
  if (letters == letters.toUpperCase()) return TextCase.lower;
  if (letters == letters.toLowerCase()) return TextCase.capitalize;
  return TextCase.upper;
}

/// The change that sets the case of [start]..[end], keeping each run's
/// attributes. Embeds and paragraph marks are retained unchanged.
Delta changeCaseDelta(Delta document, int start, int end, TextCase mode) {
  final slice = document.slice(start, end).toList();
  final pieces = [
    for (final op in slice)
      op.data is String ? (op.data as String).characters.toList() : ['\uFFFC'],
  ];
  final mapped = caseCharacters([for (final p in pieces) ...p], mode);
  final change = Delta()..retain(start);
  var at = 0;
  for (var i = 0; i < slice.length; i++) {
    final op = slice[i], count = pieces[i].length;
    final next = mapped.sublist(at, at + count).join();
    at += count;
    if (op.data is! String || next == op.data) {
      change.retain(op.length!);
      continue;
    }
    // Paragraph marks keep their block attributes; only the runs change.
    final parts = (op.data as String).split('\n');
    final nextParts = next.split('\n');
    for (var j = 0; j < parts.length; j++) {
      if (j > 0) change.retain(1);
      if (parts[j] == nextParts[j]) {
        change.retain(parts[j].length);
      } else {
        change.delete(parts[j].length);
        change.insert(nextParts[j], op.attributes);
      }
    }
  }
  return change..trim();
}

/// Applies [mode] (or the Shift+F3 cycle when null) to the selection, or to
/// the word at a collapsed caret, keeping the same range selected.
void applyTextCase(q.QuillController controller, [TextCase? mode]) {
  var selection = controller.selection;
  final delta = controller.document.toDelta();
  final plain = documentText(delta);
  if (selection.isCollapsed) {
    var s = selection.start, e = selection.start;
    while (s > 0 && _wordChar(plain[s - 1])) {
      s--;
    }
    while (e < plain.length && _wordChar(plain[e])) {
      e++;
    }
    if (s == e) return;
    selection = TextSelection(baseOffset: s, extentOffset: e);
  }
  final text = plain.substring(selection.start, selection.end);
  final chosen = mode ?? nextCaseCycle(text);
  final change = changeCaseDelta(delta, selection.start, selection.end, chosen);
  if (change.isEmpty) return;
  controller.compose(
    change,
    TextSelection(
      baseOffset: selection.start,
      extentOffset: selection.start + changeCase(text, chosen).length,
    ),
    q.ChangeSource.local,
  );
}

/// Character formatting Clear All Formatting removes. Links stay: they are
/// content, not formatting, as in Word.
const clearedCharacterAttributes = [
  'bold',
  'italic',
  'underline',
  'strike',
  'code',
  'font',
  'size',
  'color',
  'background',
  'script',
];

/// The change that clears the selection's character formatting and returns
/// its paragraphs to the document's default paragraph style (Normal), keeping
/// their list membership and Office identity. [styles] is the document's
/// style catalog, when it has one.
Delta clearFormattingDelta(
  Delta document,
  TextSelection selection, {
  Map? styles,
}) {
  final catalog = expandedBuiltinStyles(styles);
  final normal = catalog == null
      ? null
      : (catalog['defaultStyle'] as String? ?? 'Normal');
  final change = Delta();
  var offset = 0;
  for (final line in listLines(document)) {
    if (line.end < selection.start ||
        (selection.isCollapsed
            ? line.start > selection.start
            : line.start >= selection.end)) {
      continue;
    }
    // Runs inside the selection (the whole line for a collapsed caret's
    // paragraph mark is left alone: Word keeps its text formatting).
    final from = selection.start.clamp(line.start, line.end);
    final to = selection.end.clamp(line.start, line.end);
    if (from > offset) change.retain(from - offset);
    offset = from;
    for (final op in document.slice(from, to).toList()) {
      final attrs = op.attributes ?? const {};
      final cleared = {
        for (final key in clearedCharacterAttributes)
          if (attrs.containsKey(key)) key: null,
      };
      change.retain(op.length!, cleared.isEmpty ? null : cleared);
    }
    offset = to;
    if (line.end > offset) change.retain(line.end - offset);
    final metadata = listMetadata(line.attrs['token']);
    metadata.remove('tabs');
    if (normal != null) {
      metadata['paragraphFormat'] = namedParagraphFormat(catalog!, normal);
      final tabs = resolveNamedStyle(catalog, normal)['tabs'] as List;
      if (tabs.isNotEmpty) metadata['tabs'] = {'inherited': tabs, 'direct': []};
    } else {
      metadata.remove('paragraphFormat');
    }
    change.retain(1, {
      'token': metadata.isEmpty ? null : jsonEncode(metadata),
      'header': null,
      'align': null,
    });
    offset = line.end + 1;
  }
  return change..trim();
}

void applyClearFormatting(q.QuillController controller, {Map? styles}) {
  final selection = controller.selection;
  final change = clearFormattingDelta(
    controller.document.toDelta(),
    selection,
    styles: styles,
  );
  if (change.isEmpty) return;
  controller.compose(change, selection, q.ChangeSource.local);
  // Text typed at a collapsed caret starts plain too.
  if (selection.isCollapsed) {
    for (final key in clearedCharacterAttributes) {
      final attribute = q.Attribute.fromKeyValue(key, null);
      if (attribute != null) controller.formatSelection(attribute);
    }
  }
}
