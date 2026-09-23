import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';

const listFormats = {
  'decimal',
  'lowerLetter',
  'upperLetter',
  'lowerRoman',
  'upperRoman',
  'bullet',
  'none',
};
Map<String, dynamic> listMetadata(dynamic token) {
  try {
    return Map<String, dynamic>.from(jsonDecode(token as String));
  } catch (_) {
    return {};
  }
}

List<Map<String, dynamic>> defaultListLevels({
  bool bullet = false,
  int start = 1,
}) => [
  for (var i = 0; i < 9; i++)
    {
      'format': bullet
          ? 'bullet'
          : ['decimal', 'lowerLetter', 'lowerRoman'][i % 3],
      'text': bullet ? '•' : '%${i + 1}.',
      'start': i == 0 ? start : 1,
      'restart': null,
      'suffix': 'tab',
      'legal': false,
      'style': null,
      'picture': false,
    },
];
bool editableNumbering(dynamic numbering) {
  if (numbering == null) return true;
  if (numbering is! Map ||
      numbering['level'] is! int ||
      numbering['level'] < 0 ||
      numbering['level'] > 8) {
    return false;
  }
  final levels = numbering['levels'];
  return levels is List &&
      levels.length == 9 &&
      levels[numbering['level']] is Map &&
      levels.whereType<Map>().every(
        (l) => listFormats.contains(l['format']) && l['picture'] != true,
      );
}

Map<String, dynamic>? numberingForLine(Map attrs) {
  final original = listMetadata(attrs['token'])['numbering'];
  if (original is! Map || attrs['list'] == null) return null;
  final n = Map<String, dynamic>.from(jsonDecode(jsonEncode(original)));
  final level = ((attrs['indent'] as num?)?.toInt() ?? 0).clamp(0, 8);
  final levels = n['levels'] as List?;
  if (levels == null) return n;
  final current = levels[level] as Map?;
  final bullet = attrs['list'] == 'bullet';
  if (current == null || (current['format'] == 'bullet') != bullet) {
    levels[level] = defaultListLevels(bullet: bullet)[level];
  }
  n['level'] = level;
  return n;
}

String listNumber(int number, String format) {
  if (number <= 0) return '$number';
  if (format == 'lowerLetter' || format == 'upperLetter') {
    var n = number, s = '';
    while (n > 0) {
      n--;
      s = String.fromCharCode(97 + n % 26) + s;
      n ~/= 26;
    }
    return format == 'upperLetter' ? s.toUpperCase() : s;
  }
  if (format == 'lowerRoman' || format == 'upperRoman') {
    if (number > 3999) return '$number';
    var n = number, s = '';
    for (final (value, letter) in [
      (1000, 'M'),
      (900, 'CM'),
      (500, 'D'),
      (400, 'CD'),
      (100, 'C'),
      (90, 'XC'),
      (50, 'L'),
      (40, 'XL'),
      (10, 'X'),
      (9, 'IX'),
      (5, 'V'),
      (4, 'IV'),
      (1, 'I'),
    ]) {
      while (n >= value) {
        s += letter;
        n -= value;
      }
    }
    return format == 'lowerRoman' ? s.toLowerCase() : s;
  }
  return '$number';
}

/// Counters are keyed by Word numbering instance, so a list can continue across
/// an intervening paragraph without resetting. Explicit restart rules apply to
/// deeper levels; omitted rules restart after the immediately preceding level.
Map<int, String> documentListLabels(Delta delta) {
  final labels = <int, String>{};
  final counters = <String, List<int?>>{};
  var offset = 0, lineStart = 0;
  for (final op in delta.toJson()) {
    final insert = op['insert'];
    if (insert is! String) {
      offset++;
      continue;
    }
    for (var i = 0; i < insert.length; i++) {
      if (insert.codeUnitAt(i) != 10) continue;
      final attrs = op['attributes'] as Map? ?? {};
      final n = numberingForLine(attrs);
      if (n != null && editableNumbering(n)) {
        final levels = n['levels'] as List;
        final level = n['level'] as int;
        final config = levels[level] as Map;
        final id = '${n['instance'] ?? n['numId']}:${jsonEncode(levels)}';
        final counts = counters.putIfAbsent(
          id,
          () => List<int?>.filled(9, null),
        );
        counts[level] = counts[level] == null
            ? config['start'] as int? ?? 0
            : counts[level]! + 1;
        for (var j = level + 1; j < 9; j++) {
          final next = levels[j] as Map?;
          if (next == null) continue;
          final restart = next['restart'] as int? ?? j;
          if (restart > 0 && restart <= j && level == restart - 1) {
            counts[j] = null;
          }
        }
        labels[lineStart] = config['format'] == 'none'
            ? ''
            : (config['text'] as String? ?? '').replaceAllMapped(
                RegExp(r'%([1-9])'),
                (m) {
                  final index = int.parse(m[1]!) - 1;
                  if (index > level) return '';
                  final definition = levels[index] as Map?;
                  final format = config['legal'] == true
                      ? 'decimal'
                      : definition?['format'] as String? ?? 'decimal';
                  return listNumber(
                    counts[index] ?? definition?['start'] as int? ?? 0,
                    format,
                  );
                },
              );
      }
      lineStart = offset + i + 1;
    }
    offset += insert.length;
  }
  return labels;
}

List<({int start, int end, Map attrs})> listLines(Delta delta) {
  final result = <({int start, int end, Map attrs})>[];
  var offset = 0, start = 0;
  for (final op in delta.toJson()) {
    final value = op['insert'];
    if (value is! String) {
      offset++;
      continue;
    }
    for (var i = 0; i < value.length; i++) {
      if (value.codeUnitAt(i) == 10) {
        result.add((
          start: start,
          end: offset + i,
          attrs: op['attributes'] as Map? ?? {},
        ));
        start = offset + i + 1;
      }
    }
    offset += value.length;
  }
  return result;
}

/// Returns a single formatting transaction, preserving text, inline marks and
/// paragraph identity. A restart applies from this item through its contiguous
/// list; continuation can join two lists separated by prose.
Delta changeNumbering(
  Delta document,
  int at, {
  int start = 1,
  String format = 'decimal',
  bool continuePrevious = false,
  required String instance,
}) {
  final lines = listLines(document);
  final selected = lines.indexWhere((l) => at >= l.start && at <= l.end);
  if (selected < 0 || lines[selected].attrs['list'] != 'ordered') {
    throw const FormatException('Select a numbered paragraph first.');
  }
  if (start < 0 ||
      start > 2147483647 ||
      !listFormats.contains(format) ||
      format == 'bullet' ||
      format == 'none') {
    throw const FormatException('Choose a number from 0 to 2,147,483,647.');
  }
  final level = ((lines[selected].attrs['indent'] as num?)?.toInt() ?? 0).clamp(
    0,
    8,
  );
  var number =
      numberingForLine(lines[selected].attrs) ??
      {'levels': defaultListLevels(), 'level': level};
  number = Map<String, dynamic>.from(jsonDecode(jsonEncode(number)));
  final changed = <int, Map<String, dynamic>>{};
  if (continuePrevious) {
    var previous = selected - 1;
    while (previous >= 0 &&
        (lines[previous].attrs['list'] != 'ordered' ||
            (lines[previous].attrs['indent'] ?? 0) != level)) {
      previous--;
    }
    if (previous < 0) {
      throw const FormatException(
        'There is no earlier numbered list at this level.',
      );
    }
    number =
        numberingForLine(lines[previous].attrs) ??
        {'instance': instance, 'levels': defaultListLevels(), 'level': level};
    // Give the preceding native list a stable instance as well, so both
    // sequences share a DOCX numId and a counter in the editor.
    if (numberingForLine(lines[previous].attrs) == null) {
      var first = previous;
      while (first > 0 && lines[first - 1].attrs['list'] != null) {
        first--;
      }
      for (var i = first; i <= previous; i++) {
        changed[i] = number;
      }
    }
  } else {
    number.remove('numId');
    number['instance'] = instance;
    final levels = number['levels'] as List;
    final previousLevel = levels[level] as Map? ?? defaultListLevels()[level];
    levels[level] = {
      ...previousLevel,
      'start': start,
      'format': format,
      'text': previousLevel['format'] == format
          ? previousLevel['text']
          : '%${level + 1}.',
    };
  }
  String? identity(Map attrs) {
    final n = numberingForLine(attrs);
    return (n?['instance'] ?? n?['numId']) as String?;
  }

  final currentIdentity = identity(lines[selected].attrs);
  for (var i = selected; i < lines.length; i++) {
    final attrs = lines[i].attrs;
    if (attrs['list'] == null || (attrs['indent'] ?? 0) < level) break;
    if (i > selected &&
        (attrs['indent'] ?? 0) == level &&
        (attrs['list'] != 'ordered' || identity(attrs) != currentIdentity)) {
      break;
    }
    changed[i] = number;
  }
  final delta = Delta();
  var consumed = 0;
  final indexes = changed.keys.toList()..sort();
  for (final i in indexes) {
    final line = lines[i];
    final metadata = listMetadata(line.attrs['token']);
    metadata['numbering'] = {
      ...changed[i]!,
      'level': (line.attrs['indent'] ?? 0),
    };
    delta.retain(line.end - consumed);
    delta.retain(1, {'token': jsonEncode(metadata)});
    consumed = line.end + 1;
  }
  return delta;
}
