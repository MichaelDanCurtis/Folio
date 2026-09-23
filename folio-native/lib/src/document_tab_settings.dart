import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/widgets.dart';
import 'document_lists.dart';

int documentTabInterval(Map document) =>
    (document['attrs']?['defaultTabStop'] as num?)?.toInt() ?? 720;

class DocumentTabSettings extends InheritedWidget {
  final int intervalTwips;
  const DocumentTabSettings({
    required this.intervalTwips,
    required super.child,
    super.key,
  });
  static int intervalOf(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<DocumentTabSettings>()
          ?.intervalTwips ??
      720;
  @override
  bool updateShouldNotify(DocumentTabSettings oldWidget) =>
      intervalTwips != oldWidget.intervalTwips;
}

class DocumentTabStop {
  final int position;
  final String alignment, leader;
  const DocumentTabStop(
    this.position, {
    this.alignment = 'left',
    this.leader = 'none',
  });
  factory DocumentTabStop.fromJson(Map value) => DocumentTabStop(
    (value['position'] as num).toInt(),
    alignment: value['alignment'] as String? ?? 'left',
    leader: value['leader'] as String? ?? 'none',
  );
  double get pixels => position / 15;
  Map<String, dynamic> toJson() => {
    'position': position,
    'alignment': alignment,
    'leader': leader,
  };
}

List<DocumentTabStop> effectiveTabStops(dynamic tabs) {
  final result = <int, DocumentTabStop>{};
  if (tabs is Map) {
    for (final key in ['inherited', 'direct']) {
      for (final value in (tabs[key] as List? ?? []).whereType<Map>()) {
        if (value['position'] is! num) continue;
        final stop = DocumentTabStop.fromJson(value);
        if (stop.alignment == 'clear') {
          result.remove(stop.position);
        } else {
          result[stop.position] = stop;
        }
      }
    }
  }
  return result.values.toList()
    ..sort((a, b) => a.position.compareTo(b.position));
}

/// Set the effective stops while keeping each paragraph's original inheritance.
/// Explicit clears prevent removed style stops from reappearing on DOCX export.
Map<String, dynamic> replaceTabStops(
  dynamic original,
  List<DocumentTabStop> stops,
) {
  final inherited = original is Map ? original['inherited'] as List? ?? [] : [];
  final wanted = {for (final s in stops) s.position: s.toJson()};
  final inheritedByPosition = {
    for (final s in inherited.whereType<Map>()) s['position']: s,
  };
  final direct = <Map<String, dynamic>>[];
  for (final p in inheritedByPosition.keys) {
    if (!wanted.containsKey(p)) {
      direct.add(DocumentTabStop(p as int, alignment: 'clear').toJson());
    }
  }
  for (final s in stops) {
    if (jsonEncode(s.toJson()) != jsonEncode(inheritedByPosition[s.position])) {
      direct.add(s.toJson());
    }
  }
  direct.sort((a, b) => (a['position'] as int).compareTo(b['position'] as int));
  return {'inherited': inherited, 'direct': direct};
}

/// One formatting transaction across every selected ordinary paragraph.
/// A selection ending at the next paragraph start does not format that paragraph.
Delta changeTabStops(
  Delta document,
  TextSelection selection,
  List<DocumentTabStop> stops,
) {
  final change = Delta();
  var offset = 0;
  for (final line in listLines(document)) {
    if (line.end < selection.start ||
        (selection.isCollapsed
            ? line.start > selection.start
            : line.start >= selection.end)) {
      continue;
    }
    final body = document.slice(line.start, line.end).toList();
    if (body.any((op) => op.data is! String)) {
      throw const FormatException(
        'Select document paragraphs outside the table or preserved object.',
      );
    }
    final metadata = listMetadata(line.attrs['token']);
    if (jsonEncode(
          effectiveTabStops(metadata['tabs']).map((s) => s.toJson()).toList(),
        ) ==
        jsonEncode(stops.map((s) => s.toJson()).toList())) {
      continue;
    }
    metadata['tabs'] = replaceTabStops(metadata['tabs'], stops);
    change.retain(line.end - offset);
    change.retain(1, {'token': jsonEncode(metadata)});
    offset = line.end + 1;
  }
  return change..trim();
}
