import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_codec.dart';

const proofreadingLanguages = {
  'en-US': 'English (US)',
  'en-GB': 'English (UK)',
  'en-CA': 'English (Canada)',
  'en-AU': 'English (Australia)',
  'en-IN': 'English (India)',
};

class WritingIssue {
  final int start, end;
  final String text, category, message;
  final List<String> suggestions;
  WritingIssue(Map data, [int offset = 0])
    : start = (data['start'] as int) + offset,
      end = (data['end'] as int) + offset,
      text = data['text'] as String,
      category = data['category'] as String,
      message = data['message'] as String,
      suggestions = List<String>.from(data['suggestions']);
  String get key => '$start:$end:$text:$message';
  Color get color => category == 'spelling'
      ? const Color(0xffc4314b)
      : category == 'grammar'
      ? const Color(0xff185abd)
      : const Color(0xff8764b8);
}

/// Ephemeral decoration only. Diagnostics never enter document formatting or exports.
class ProofreadingController extends ChangeNotifier {
  final q.QuillController editor;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>) check;
  final String? settingsPath;
  final _cache = <String, Map<String, dynamic>>{};
  final _ignored = <String>{};
  final words = <String>{};
  List<WritingIssue> issues = [];
  String language = 'en-US', _signature = '', checkedText = '';
  String? error, settingsError;
  bool enabled = true,
      grammar = true,
      style = false,
      busy = false,
      partial = false;
  bool _disposed = false, _running = false;
  int _generation = 0;
  Timer? _timer;
  Future<void> _saving = Future.value();
  ProofreadingController(
    this.editor, {
    required this.check,
    this.settingsPath,
  }) {
    editor.addListener(_changed);
    unawaited(_load());
  }
  Future<void> _load() async {
    try {
      if (settingsPath != null && await File(settingsPath!).exists()) {
        final data =
            jsonDecode(await File(settingsPath!).readAsString()) as Map;
        if (_disposed) return;
        language = proofreadingLanguages.containsKey(data['language'])
            ? data['language']
            : 'en-US';
        enabled = data['enabled'] != false;
        grammar = data['grammar'] != false;
        style = data['style'] == true;
        words.addAll(List<String>.from(data['words'] ?? []).take(10000));
      }
    } catch (_) {
      settingsError =
          'Your saved dictionary could not be read. It has been left intact.';
    }
    if (!_disposed) _changed(force: true);
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  void _changed({bool force = false}) {
    if (_disposed) return;
    final signature = jsonEncode(editor.document.toDelta().toJson());
    if (!force && signature == _signature) return;
    _signature = signature;
    _generation++;
    _timer?.cancel();
    issues = [];
    _ignored.clear();
    checkedText = '';
    error = null;
    busy = enabled;
    _notify();
    if (enabled) {
      _timer = Timer(
        const Duration(milliseconds: 650),
        () => unawaited(checkNow()),
      );
    }
  }

  Future<void> configure({
    String? language,
    bool? enabled,
    bool? grammar,
    bool? style,
  }) async {
    if (language != null && proofreadingLanguages.containsKey(language)) {
      this.language = language;
    }
    this.enabled = enabled ?? this.enabled;
    this.grammar = grammar ?? this.grammar;
    this.style = style ?? this.style;
    _cache.clear();
    _changed(force: true);
    await saveSettings();
  }

  Future<void> saveSettings() {
    if (settingsPath == null) return Future.value();
    if (settingsError != null) return Future.error(StateError(settingsError!));
    final data = jsonEncode({
      'language': language,
      'enabled': enabled,
      'grammar': grammar,
      'style': style,
      'words': words.toList()..sort(),
    });
    final next = _saving.then((_) async {
      final file = File('$settingsPath.tmp');
      await file.parent.create(recursive: true);
      await file.writeAsString(data, flush: true);
      await file.rename(settingsPath!);
    });
    _saving = next.catchError((Object e) {
      error = 'Dictionary settings could not be saved: $e';
      _notify();
    });
    return next;
  }

  Future<void> flushSettings() => _saving;

  Future<void> learn(String word) async {
    if (word.length > 100 || words.length >= 10000) {
      throw StateError('Personal dictionary limit reached.');
    }
    words.add(word.toLowerCase());
    _changed(force: true);
    await saveSettings();
  }

  Future<void> forget(String word) async {
    words.remove(word);
    _changed(force: true);
    await saveSettings();
  }

  void ignore(WritingIssue issue) {
    _ignored.add(issue.key);
    issues.removeWhere((i) => i.key == issue.key);
    _notify();
  }

  bool isCurrent() =>
      checkedText == editor.document.toPlainText() &&
      _signature == jsonEncode(editor.document.toDelta().toJson());

  /// Mask code, links, embedded objects and URLs without changing UTF-16 offsets.
  static String checkableText(q.Document document) {
    final out = StringBuffer();
    final codeLines = <int>[];
    for (final op in document.toDelta().toList()) {
      final value = op.data;
      if (value is! String) {
        out.write(' ');
        continue;
      }
      final attrs = op.attributes ?? {};
      if (attrs.containsKey('code-block')) {
        for (var i = 0; i < value.length; i++) {
          if (value[i] == '\n') codeLines.add(out.length + i);
        }
      }
      out.write(
        attrs.containsKey('code') ||
                attrs.containsKey('code-block') ||
                attrs.containsKey('link')
            ? value.replaceAll(RegExp(r'[^\n]'), ' ')
            : value,
      );
    }
    var masked = out.toString();
    for (final end in codeLines) {
      final start = end == 0 ? 0 : masked.lastIndexOf('\n', end - 1) + 1;
      masked = masked.replaceRange(start, end, ' ' * (end - start));
    }
    return masked.replaceAllMapped(
      RegExp(r'https?://\S+|www\.\S+|[\w.+-]+@[\w.-]+\.[A-Za-z]{2,}'),
      (m) => ' ' * m.group(0)!.length,
    );
  }

  Future<void> checkNow() async {
    _timer?.cancel();
    if (_disposed || !enabled || _running) return;
    _running = true;
    final generation = _generation;
    final plain = editor.document.toPlainText();
    final masked = checkableText(editor.document);
    busy = true;
    partial = false;
    error = null;
    _notify();
    final found = <WritingIssue>[];
    try {
      var offset = 0;
      for (final paragraph in masked.split('\n')) {
        if (_disposed || generation != _generation) return;
        if (offset > 100000 || paragraph.length > 12000) {
          partial = true;
        } else if (paragraph.trim().isNotEmpty) {
          var result = _cache[paragraph];
          if (result == null) {
            result = await check({
              'op': 'proofread',
              'text': paragraph,
              'language': language,
            });
            if (_disposed || generation != _generation) return;
            if (_cache.length >= 300) _cache.remove(_cache.keys.first);
            _cache[paragraph] = result;
          }
          partial = partial || result['truncated'] == true;
          for (final data in result['issues'] as List) {
            final issue = WritingIssue(data as Map, offset);
            if (issue.end > plain.length ||
                issue.start < 0 ||
                plain.substring(issue.start, issue.end) != issue.text) {
              continue;
            }
            if (issue.category == 'spelling' &&
                words.contains(issue.text.toLowerCase())) {
              continue;
            }
            if (issue.category == 'grammar' && !grammar) continue;
            if (issue.category == 'style' && !style) continue;
            if (!_ignored.contains(issue.key)) found.add(issue);
          }
        }
        offset += paragraph.length + 1;
      }
      if (!_disposed && generation == _generation) {
        issues = found;
        checkedText = plain;
      }
    } catch (e) {
      if (!_disposed && generation == _generation) {
        error = 'Writing checks are unavailable: $e';
      }
    } finally {
      _running = false;
      if (!_disposed) {
        if (generation == _generation) {
          busy = false;
          _notify();
        } else if (enabled) {
          // An edit may already have scheduled a debounce while this request
          // was in flight. Do not orphan that timer when scheduling the retry.
          _timer?.cancel();
          _timer = Timer(
            const Duration(milliseconds: 100),
            () => unawaited(checkNow()),
          );
        }
      }
    }
  }

  InlineSpan spanBuilder(
    BuildContext context,
    q.Node node,
    int offset,
    String text,
    TextStyle? style,
    GestureRecognizer? recognizer,
  ) {
    final start = node.documentOffset + offset;
    // Diagnostics are ordered and non-overlapping. Search only the visible run.
    var low = 0, high = issues.length;
    while (low < high) {
      final mid = (low + high) ~/ 2;
      if (issues[mid].end <= start) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    final matching = <WritingIssue>[];
    for (
      var i = low;
      i < issues.length && issues[i].start < start + text.length;
      i++
    ) {
      matching.add(issues[i]);
    }
    if (matching.isEmpty) {
      return TextSpan(text: text, style: style, recognizer: recognizer);
    }
    final spans = <InlineSpan>[];
    var cursor = 0;
    for (final issue in matching) {
      final a = (issue.start - start).clamp(cursor, text.length);
      final b = (issue.end - start).clamp(a, text.length);
      if (a > cursor) {
        spans.add(
          TextSpan(text: text.substring(cursor, a), recognizer: recognizer),
        );
      }
      spans.add(
        TextSpan(
          text: text.substring(a, b),
          recognizer: recognizer,
          style: TextStyle(
            decoration: TextDecoration.combine([
              if (style?.decoration != null) style!.decoration!,
              TextDecoration.underline,
            ]),
            decorationStyle: TextDecorationStyle.wavy,
            decorationColor: issue.color,
            decorationThickness: 1.3,
          ),
        ),
      );
      cursor = b;
    }
    if (cursor < text.length) {
      spans.add(TextSpan(text: text.substring(cursor), recognizer: recognizer));
    }
    return TextSpan(style: style, children: spans);
  }

  @override
  void dispose() {
    _disposed = true;
    _generation++;
    _timer?.cancel();
    editor.removeListener(_changed);
    super.dispose();
  }
}

/// Text edits are validated together before a copy is changed; original document
/// and its formatting remain untouched until the caller commits the result.
Map<String, dynamic> applyWritingEdits(
  q.QuillController editor,
  List<Map<String, dynamic>> edits,
) {
  if (edits.isEmpty || edits.length > 200) {
    throw StateError('Provide between 1 and 200 corrections.');
  }
  final text = editor.document.toPlainText();
  final checkable = ProofreadingController.checkableText(editor.document);
  final sorted = [...edits]
    ..sort((a, b) => (a['start'] as int).compareTo(b['start'] as int));
  var previousEnd = -1;
  bool boundary(int index) =>
      index <= 0 ||
      index >= text.length ||
      !(text.codeUnitAt(index) >= 0xdc00 && text.codeUnitAt(index) <= 0xdfff);
  for (final edit in sorted) {
    final start = edit['start'] as int, end = edit['end'] as int;
    final expected = edit['expectedText'] as String,
        replacement = edit['replacement'] as String;
    if (start < 0 ||
        end <= start ||
        end >= text.length ||
        start < previousEnd ||
        !boundary(start) ||
        !boundary(end) ||
        text.substring(start, end) != expected ||
        checkable.substring(start, end) != expected ||
        expected.contains(RegExp('[\n\uFFFC]')) ||
        replacement.contains(RegExp('[\r\n\uFFFC]')) ||
        replacement.length > 12000) {
      throw StateError(
        'This correction no longer matches the document, overlaps another edit, or crosses a paragraph. Check again.',
      );
    }
    previousEnd = end;
  }
  final copy = q.QuillController(
    document: q.Document.fromDelta(editor.document.toDelta()),
    selection: const TextSelection.collapsed(offset: 0),
  );
  try {
    for (final edit in sorted.reversed) {
      // Keep matching prefix/suffix characters in their original rich-text runs.
      // For example "an letter" -> "a letter" only removes the extra "n".
      final before = (edit['expectedText'] as String).runes.toList();
      final after = (edit['replacement'] as String).runes.toList();
      var prefix = 0, suffix = 0;
      while (prefix < before.length &&
          prefix < after.length &&
          before[prefix] == after[prefix]) {
        prefix++;
      }
      while (suffix < before.length - prefix &&
          suffix < after.length - prefix &&
          before[before.length - 1 - suffix] ==
              after[after.length - 1 - suffix]) {
        suffix++;
      }
      final start =
          (edit['start'] as int) +
          String.fromCharCodes(before.take(prefix)).length;
      final remove = String.fromCharCodes(
        before.sublist(prefix, before.length - suffix),
      ).length;
      final insert = String.fromCharCodes(
        after.sublist(prefix, after.length - suffix),
      );
      copy.replaceText(start, remove, insert, null);
    }
    return deltaToDocument(copy.document.toDelta());
  } finally {
    copy.dispose();
  }
}

class WritingReviewSnapshot {
  final int revision;
  final String workspaceId, plain;
  final List<Map<String, dynamic>> blocks;
  final bool partial;
  WritingReviewSnapshot._(
    this.revision,
    this.workspaceId,
    this.plain,
    this.blocks,
    this.partial,
  );
  factory WritingReviewSnapshot.capture(
    q.QuillController editor,
    int revision,
    String workspaceId, {
    int? start,
    int? end,
  }) {
    final plain = editor.document.toPlainText();
    final checkable = ProofreadingController.checkableText(editor.document);
    final blocks = <Map<String, dynamic>>[];
    var offset = 0, total = 0;
    var partial = false;
    for (final paragraph in plain.split('\n')) {
      final a = offset.clamp(start ?? 0, end ?? plain.length);
      final b = (offset + paragraph.length).clamp(
        start ?? 0,
        end ?? plain.length,
      );
      if (a < b && !plain.substring(a, b).contains('\uFFFC')) {
        final text = checkable.substring(a, b);
        if (text.trim().isEmpty) {
          offset += paragraph.length + 1;
          continue;
        }
        if (total + text.length <= 60000 && text.length <= 12000) {
          blocks.add({'id': 'p$a', 'start': a, 'end': b, 'text': text});
          total += text.length;
        } else {
          partial = true;
        }
      }
      offset += paragraph.length + 1;
    }
    return WritingReviewSnapshot._(
      revision,
      workspaceId,
      plain,
      blocks,
      partial,
    );
  }
  Map<String, dynamic> toJson() => {
    'blocks': blocks,
    'baseRevision': revision,
    'partial': partial,
    'instructions':
        'Use proofreadEdits with blockId, a unique exact expectedText inside that block, and replacement. Make small corrections; preserve meaning, voice, facts and figures. Embedded objects are excluded.',
  };
  List<Map<String, dynamic>> resolve(
    q.QuillController editor,
    int currentRevision,
    String currentId,
    List<dynamic> edits,
  ) {
    if (currentRevision != revision ||
        currentId != workspaceId ||
        editor.document.toPlainText() != plain) {
      throw StateError(
        'The document changed after review. Read the workspace again.',
      );
    }
    return edits.map((e) {
      final block = blocks.where((b) => b['id'] == e['blockId']).firstOrNull;
      final expected = e['expectedText'] as String;
      if (block == null || expected.isEmpty) {
        throw StateError('Unknown review paragraph or empty correction.');
      }
      final text = block['text'] as String;
      final index = text.indexOf(expected);
      if (index < 0 || text.indexOf(expected, index + 1) >= 0) {
        throw StateError(
          'Use an exact, unique phrase from the review paragraph.',
        );
      }
      return <String, dynamic>{
        'start': (block['start'] as int) + index,
        'end': (block['start'] as int) + index + expected.length,
        'expectedText': expected,
        'replacement': e['replacement'] as String,
      };
    }).toList();
  }
}
