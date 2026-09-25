// Integrity checks for the parity status register (docs/status and
// docs/STATUS.md). The register is Markdown for people; these checks keep
// its IDs, states, evidence links and summary counts consistent.
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

const states = [
  'Specified',
  'Implemented',
  'Automated verified',
  'Native verified',
  'Office compared',
  'Accepted',
];
const areaFiles = [
  'docs/status/desktop.md',
  'docs/status/word.md',
  'docs/status/excel.md',
];

final families = [
  for (var i = 1; i <= 10; i++) 'D${i.toString().padLeft(2, '0')}',
  for (var i = 1; i <= 17; i++) 'W${i.toString().padLeft(2, '0')}',
  for (var i = 1; i <= 19; i++) 'X${i.toString().padLeft(2, '0')}',
];

class Child {
  Child(this.file, this.family, this.cells);
  final String file;
  final String family;
  final List<String> cells;
  String get id => cells[0];
  String get milestone => cells[2];
  String get state => cells[3];
  String get evidence => cells[4];
}

/// Cells of a Markdown table row, trimmed; escaped pipes stay in the text.
List<String> cellsOf(String line) {
  final body = line.trim();
  final inner = body.substring(1, body.endsWith('|') ? body.length - 1 : null);
  return inner.split(RegExp(r'(?<!\\)\|')).map((cell) => cell.trim()).toList();
}

/// The register's families and children, plus any structural problems found
/// while reading it (reported by a test, since this runs outside one).
({Map<String, String> titles, List<Child> children, List<String> problems})
readRegister() {
  final titles = <String, String>{};
  final children = <Child>[];
  final problems = <String>[];
  for (final path in areaFiles) {
    String? family;
    for (final line in File(path).readAsLinesSync()) {
      final heading = RegExp(r'^## ([DWX]\d\d) (.+)$').firstMatch(line);
      if (heading != null) {
        family = heading.group(1)!;
        if (titles.containsKey(family)) problems.add('$family twice');
        titles[family] = heading.group(2)!.trim();
        continue;
      }
      if (line.startsWith('## ')) family = null;
      if (!line.startsWith('|')) continue;
      final cells = cellsOf(line);
      if (!RegExp(r'^[DWX]\d\d\.\d\d$').hasMatch(cells.first)) continue;
      if (family == null) {
        problems.add('${cells.first} outside a family');
        continue;
      }
      children.add(Child(path, family, cells));
    }
  }
  return (titles: titles, children: children, problems: problems);
}

/// GitHub's heading anchors for a Markdown file: lower case, punctuation
/// dropped, spaces as hyphens, and `-1`, `-2` suffixes for repeats.
Set<String> anchorsOf(String path) {
  final anchors = <String>{};
  final seen = <String, int>{};
  var fenced = false;
  for (final line in File(path).readAsLinesSync()) {
    if (line.startsWith('```')) fenced = !fenced;
    final heading = RegExp(r'^#{1,6} (.+)$').firstMatch(line);
    if (fenced || heading == null) continue;
    final slug = heading
        .group(1)!
        .trim()
        .toLowerCase()
        .replaceAll(RegExp(r'[^\p{L}\p{N} _-]', unicode: true), '')
        .replaceAll(' ', '-');
    final count = seen[slug] ?? 0;
    anchors.add(count == 0 ? slug : '$slug-$count');
    seen[slug] = count + 1;
  }
  return anchors;
}

void main() {
  late final ({
    Map<String, String> titles,
    List<Child> children,
    List<String> problems,
  })
  register;
  setUpAll(() => register = readRegister());

  test('every parity family is in the register with children', () {
    expect(register.problems, isEmpty);
    expect(register.titles.keys.toSet(), families.toSet());
    for (final family in families) {
      expect(
        register.children.where((c) => c.family == family),
        isNotEmpty,
        reason: family,
      );
    }
  });

  test('child rows are well formed', () {
    final seen = <String>{};
    for (final child in register.children) {
      final where = '${child.file}: ${child.id}';
      expect(child.cells, hasLength(6), reason: where);
      expect(child.id.startsWith('${child.family}.'), isTrue, reason: where);
      expect(seen.add(child.id), isTrue, reason: '$where is a duplicate');
      expect(child.cells[1], isNotEmpty, reason: where);
      expect(
        RegExp(r'^M[1-5]$').hasMatch(child.milestone),
        isTrue,
        reason: '$where milestone "${child.milestone}"',
      );
      expect(
        [...states, 'Retired'].contains(child.state),
        isTrue,
        reason: '$where state "${child.state}"',
      );
      expect(child.cells[5], isNotEmpty, reason: '$where needs a next step');
    }
  });

  test('relative evidence links and anchors resolve', () {
    final link = RegExp(r'\]\(([^)#\s]+)(?:#([^)\s]*))?\)');
    final files = [
      ...areaFiles,
      'docs/STATUS.md',
      'docs/status/README.md',
      'docs/status/CORPUS.md',
      'docs/status/OFFICE_BASELINE.md',
    ];
    for (final path in files) {
      final dir = File(path).parent.path;
      // Links inside fenced code blocks are examples, not links.
      final text = File(
        path,
      ).readAsStringSync().replaceAll(RegExp(r'```.*?```', dotAll: true), '');
      for (final match in link.allMatches(text)) {
        final target = match.group(1)!;
        if (target.contains('://') || target.startsWith('mailto:')) continue;
        final resolved = '$dir/$target';
        final exists =
            File(resolved).existsSync() || Directory(resolved).existsSync();
        expect(exists, isTrue, reason: '$path links to missing $target');
        final anchor = match.group(2);
        if (anchor != null && anchor.isNotEmpty && target.endsWith('.md')) {
          expect(
            anchorsOf(resolved).contains(anchor),
            isTrue,
            reason: '$path links to missing anchor $target#$anchor',
          );
        }
      }
    }
  });

  test('STATUS.md counts match the register', () {
    final rows = <String, List<int>>{};
    for (final line in File('docs/STATUS.md').readAsLinesSync()) {
      if (!line.startsWith('|')) continue;
      final cells = cellsOf(line);
      final key = cells.first.replaceAll('*', '');
      if (!RegExp(r'^([DWX]\d\d|All)$').hasMatch(key)) continue;
      if (cells.length != 2 + states.length + 1) continue;
      rows[key] = [
        for (final cell in cells.skip(2)) int.parse(cell.replaceAll('*', '')),
      ];
    }
    List<int> count(Iterable<Child> children) {
      final live = children.where((c) => c.state != 'Retired');
      return [
        for (final state in states) live.where((c) => c.state == state).length,
        live.length,
      ];
    }

    for (final family in families) {
      expect(
        rows[family],
        count(register.children.where((c) => c.family == family)),
        reason: '$family counts in STATUS.md',
      );
    }
    expect(rows['All'], count(register.children), reason: 'All counts');
  });
}
