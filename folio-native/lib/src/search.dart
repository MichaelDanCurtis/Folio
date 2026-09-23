import 'dart:convert';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'document_codec.dart';
import 'sheet_view.dart' show formatCellValue;
import 'workbook_block.dart' show parseCell;

class FindIntent extends Intent {
  final bool replace;
  const FindIntent([this.replace = false]);
}

class FindNextIntent extends Intent {
  final bool backwards;
  const FindNextIntent([this.backwards = false]);
}

const searchShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.keyF, meta: true): FindIntent(),
  SingleActivator(LogicalKeyboardKey.keyF, control: true): FindIntent(),
  SingleActivator(LogicalKeyboardKey.keyH, meta: true, shift: true): FindIntent(
    true,
  ),
  SingleActivator(LogicalKeyboardKey.keyH, control: true): FindIntent(true),
  SingleActivator(LogicalKeyboardKey.keyG, meta: true): FindNextIntent(),
  SingleActivator(LogicalKeyboardKey.keyG, meta: true, shift: true):
      FindNextIntent(true),
  SingleActivator(LogicalKeyboardKey.f3): FindNextIntent(),
  SingleActivator(LogicalKeyboardKey.f3, shift: true): FindNextIntent(true),
};

class SearchOptions {
  final bool matchCase, wholeWords, entireCell, values, allSheets, byColumns;
  const SearchOptions({
    this.matchCase = false,
    this.wholeWords = false,
    this.entireCell = false,
    this.values = false,
    this.allSheets = false,
    this.byColumns = false,
  });
}

// All offsets stay in UTF-16, the same coordinate system as Flutter/Quill.
final _word = RegExp(r'[\p{L}\p{N}\p{M}\p{Pc}]', unicode: true);
bool _wordBefore(String text, int offset) {
  if (offset == 0) return false;
  final last = text.codeUnitAt(offset - 1);
  return _word.hasMatch(
    text.substring(
      offset - (last >= 0xdc00 && last <= 0xdfff && offset >= 2 ? 2 : 1),
      offset,
    ),
  );
}

bool _wordAfter(String text, int offset) {
  if (offset == text.length) return false;
  final first = text.codeUnitAt(offset);
  return _word.hasMatch(
    text.substring(
      offset,
      offset +
          (first >= 0xd800 && first <= 0xdbff && offset + 1 < text.length
              ? 2
              : 1),
    ),
  );
}

Iterable<Match> textMatches(
  String text,
  String query,
  SearchOptions options,
) sync* {
  if (query.isEmpty ||
      query.contains('\n') ||
      query.contains('\r') ||
      query.contains('\uFFFC')) {
    return;
  }
  final pattern = RegExp(
    RegExp.escape(query),
    caseSensitive: options.matchCase,
    unicode: true,
  );
  for (final match in pattern.allMatches(text)) {
    if (options.entireCell && (match.start != 0 || match.end != text.length)) {
      continue;
    }
    if (options.wholeWords &&
        (_wordBefore(text, match.start) || _wordAfter(text, match.end))) {
      continue;
    }
    yield match;
  }
}

String _inlineText(Map node) => ((node['content'] as List?) ?? [])
    .map(
      (n) => n['type'] == 'text'
          ? n['text']
          : n['type'] == 'hardBreak'
          ? '\n'
          : '\uFFFC',
    )
    .join();

class DocumentSearchBlock {
  final List<int> path;
  final String text, label;
  final int offset, heading;
  final bool embedded;
  const DocumentSearchBlock(
    this.path,
    this.text,
    this.label,
    this.offset,
    this.embedded,
    this.heading,
  );
}

class SearchHit {
  final String label, text;
  final int start, end;
  final DocumentSearchBlock? block;
  final String? sheetId, address, raw;
  const SearchHit({
    required this.label,
    required this.text,
    required this.start,
    required this.end,
    this.block,
    this.sheetId,
    this.address,
    this.raw,
  });
  int get documentStart => block!.offset + (block!.embedded ? 0 : start);
  int get documentEnd => block!.offset + (block!.embedded ? 1 : end);
  String get identity =>
      block == null ? '$sheetId!$address' : '${block!.path.join('.')}:$start';
  String get excerpt {
    // Rune iteration prevents a preview from splitting an emoji surrogate pair.
    final before = text.substring(0, start).runes.toList();
    final after = text.substring(end).runes.toList();
    return '${before.length > 28 ? '…' : ''}${String.fromCharCodes(before.skip(before.length > 28 ? before.length - 28 : 0))}'
        '${text.substring(start, end)}${String.fromCharCodes(after.take(60))}${after.length > 60 ? '…' : ''}';
  }
}

class SearchResults {
  static const limit = 10000;
  final List<SearchHit> hits;
  final bool truncated;
  const SearchResults(this.hits, [this.truncated = false]);
}

class DocumentSearchIndex {
  final String fingerprint;
  final List<DocumentSearchBlock> blocks;
  DocumentSearchIndex._(this.fingerprint, this.blocks);
  factory DocumentSearchIndex(Map document) {
    final blocks = <DocumentSearchBlock>[];
    var topOffset = 0, paragraph = 0;
    final nodes = (document['content'] as List?) ?? [];
    for (var top = 0; top < nodes.length; top++) {
      final delta = documentToDelta({
        'content': [nodes[top]],
      });
      final embedded =
          delta.first.data is Map &&
          (delta.first.data as Map).containsKey('folioNode');
      var innerOffset = topOffset;
      void visit(Map node, List<int> path, String? container) {
        final type = node['type'];
        if (type == 'paragraph' || type == 'heading') {
          paragraph++;
          final text = _inlineText(node);
          final level = type == 'heading'
              ? (node['attrs']?['level'] as int? ?? 1)
              : 0;
          blocks.add(
            DocumentSearchBlock(
              List.unmodifiable(path),
              text,
              container ??
                  (level > 0 ? 'Heading $level' : 'Paragraph $paragraph'),
              innerOffset,
              embedded,
              level,
            ),
          );
          if (!embedded) innerOffset += text.length + 1;
          return;
        }
        // Unknown objects are preserved, not reinterpreted as editable prose.
        if (![
          'bulletList',
          'orderedList',
          'listItem',
          'blockquote',
          'table',
          'tableRow',
          'tableCell',
          'tableHeader',
        ].contains(type)) {
          return;
        }
        final children = (node['content'] as List?) ?? [];
        for (var i = 0; i < children.length; i++) {
          final label = type == 'tableRow'
              ? '$container · cell ${i + 1}'
              : type == 'table'
              ? 'Table ${top + 1} · row ${i + 1}'
              : container;
          visit(children[i], [...path, i], label);
        }
      }

      visit(nodes[top], [top], null);
      topOffset += delta.toList().fold<int>(0, (sum, op) => sum + op.length!);
    }
    return DocumentSearchIndex._(jsonEncode(document), blocks);
  }

  SearchResults find(String query, SearchOptions options) {
    final hits = <SearchHit>[];
    for (final block in blocks) {
      for (final match in textMatches(block.text, query, options)) {
        if (hits.length == SearchResults.limit) {
          return SearchResults(hits, true);
        }
        hits.add(
          SearchHit(
            label: block.label,
            text: block.text,
            start: match.start,
            end: match.end,
            block: block,
          ),
        );
      }
    }
    return SearchResults(hits);
  }

  Map<String, dynamic> replace(
    Map current,
    List<SearchHit> hits,
    String replacement,
  ) {
    if (jsonEncode(current) != fingerprint) {
      throw StateError('The document changed. Search again before replacing.');
    }
    if (replacement.contains('\n') ||
        replacement.contains('\r') ||
        replacement.contains('\uFFFC')) {
      throw StateError('Replace with a single line of text.');
    }
    if (replacement.length > 10000) {
      throw StateError('Replacement text is too long.');
    }
    final copy = jsonDecode(fingerprint) as Map<String, dynamic>;
    final groups = <String, List<SearchHit>>{};
    for (final hit in hits) {
      if (!blocks.contains(hit.block)) {
        throw StateError('The search result belongs to another document.');
      }
      groups.putIfAbsent(hit.block!.path.join('.'), () => []).add(hit);
    }
    for (final group in groups.values) {
      Map node = copy;
      for (final index in group.first.block!.path) {
        node = node['content'][index];
      }
      group.sort((a, b) => b.start.compareTo(a.start));
      var lastStart = _inlineText(node).length;
      for (final hit in group) {
        if (hit.end > lastStart ||
            hit.start < 0 ||
            hit.start >= hit.end ||
            _inlineText(node).substring(hit.start, hit.end) !=
                hit.text.substring(hit.start, hit.end)) {
          throw StateError(
            'The search results overlap or changed. Search again.',
          );
        }
        if (hit.text.substring(hit.start, hit.end) != replacement) {
          _replaceInline(node, hit.start, hit.end, replacement);
        }
        lastStart = hit.start;
      }
    }
    return copy;
  }
}

void _replaceInline(Map node, int start, int end, String replacement) {
  var offset = 0, inserted = false;
  final output = <dynamic>[];
  for (final child in node['content'] as List) {
    final text = child['type'] == 'text' ? child['text'] as String : null;
    final length = text?.length ?? 1, next = offset + (text?.length ?? 1);
    if (next <= start || offset >= end) {
      output.add(child);
    } else {
      if (text == null) {
        throw StateError('A replacement cannot cross an embedded object.');
      }
      final localStart = (start - offset).clamp(0, length);
      final localEnd = (end - offset).clamp(0, length);
      if (localStart > 0) {
        output.add({...child, 'text': text.substring(0, localStart)});
      }
      if (!inserted && replacement.isNotEmpty) {
        output.add({...child, 'text': replacement});
      }
      inserted = true;
      if (localEnd < length) {
        output.add({...child, 'text': text.substring(localEnd)});
      }
    }
    offset = next;
  }
  node['content'] = output;
}

SearchResults findInWorkbook(
  List sheets,
  Map calculated,
  String activeSheet,
  String query,
  SearchOptions options, {
  Map formatted = const {},
}) {
  final hits = <SearchHit>[];
  if (query.isEmpty) return const SearchResults([]);
  for (final sheet in sheets) {
    if (!options.allSheets && sheet['id'] != activeSheet) continue;
    final cells = sheet['cells'] as Map;
    final addresses = cells.keys.cast<String>().toList()
      ..sort((a, b) {
        final (ar, ac) = parseCell(a);
        final (br, bc) = parseCell(b);
        final primary = options.byColumns ? ac.compareTo(bc) : ar.compareTo(br);
        return primary != 0
            ? primary
            : options.byColumns
            ? ar.compareTo(br)
            : ac.compareTo(bc);
      });
    for (final address in addresses) {
      final raw = cells[address].toString();
      final text = options.values
          ? formatCellValue(
              calculated[sheet['id']]?[address] ?? raw,
              (sheet['cellStyles']?[address] as Map?) ?? {},
              display: formatted[sheet['id']]?[address] as Map?,
            )
          : raw.startsWith("'")
          ? raw.substring(1)
          : raw;
      final matches = textMatches(text, query, options);
      if (matches.isEmpty) continue;
      if (hits.length == SearchResults.limit) return SearchResults(hits, true);
      final first = matches.first;
      hits.add(
        SearchHit(
          label: '${sheet['name']}!$address',
          text: text,
          start: first.start,
          end: first.end,
          sheetId: sheet['id'],
          address: address,
          raw: raw,
        ),
      );
    }
  }
  return SearchResults(hits);
}

List<Map<String, dynamic>> workbookReplacements(
  List sheets,
  List<SearchHit> hits,
  String query,
  String replacement,
  SearchOptions options,
) {
  if (options.values) {
    throw StateError(
      'Switch to Formulas & text to replace editable cell content.',
    );
  }
  final edits = <Map<String, dynamic>>[];
  for (final hit in hits) {
    final sheet = sheets.firstWhere((s) => s['id'] == hit.sheetId);
    if (sheet['cells'][hit.address] != hit.raw) {
      throw StateError('The cell changed. Search again before replacing.');
    }
    final quoted = hit.raw!.startsWith("'");
    var value = quoted ? hit.raw!.substring(1) : hit.raw!;
    final matches = textMatches(value, query, options).toList();
    for (final match in matches.reversed) {
      value = value.replaceRange(match.start, match.end, replacement);
    }
    if (quoted && value.isNotEmpty) value = "'$value";
    if (value != hit.raw) {
      edits.add({
        'sheetId': hit.sheetId,
        'address': hit.address,
        'value': value,
      });
    }
  }
  return edits;
}
