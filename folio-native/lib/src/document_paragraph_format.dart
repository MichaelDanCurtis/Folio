import 'document_text_style.dart';
import 'dart:convert';
import 'dart:math' as math;
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart' show mapEquals;
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_lists.dart';
import 'document_styles.dart';

/// Word uses 0–8 for outline levels 1–9 and 9 for body text.
int? paragraphOutlineLevel(dynamic format) {
  if (format is! Map) return null;
  final value =
      format['direct']?['outlineLevel'] ??
      format['inheritedOutlineLevel'] ??
      format['inherited']?['outlineLevel'];
  return value is int && value >= 0 && value <= 9 ? value : null;
}

Map<String, dynamic> paragraphFormatForAttrs(
  Map attrs, {
  q.DefaultStyles? defaults,
}) {
  final stored = listMetadata(attrs['token'])['paragraphFormat'];
  if (stored is Map) return Map<String, dynamic>.from(stored);
  final styles = defaults ?? documentEditorStyles();
  final style =
      switch (attrs['header']) {
        1 => styles.h1,
        2 => styles.h2,
        3 => styles.h3,
        4 => styles.h4,
        5 => styles.h5,
        6 => styles.h6,
        _ => styles.paragraph,
      } ??
      styles.paragraph;
  // A new native paragraph has no source style in DOCX. Materialize its current
  // layout on first formatting so an indent-only edit survives export/reimport.
  return {
    'inherited': <String, dynamic>{},
    'direct': {
      'before': ((style?.verticalSpacing.top ?? 0) * 15).round(),
      'after': ((style?.verticalSpacing.bottom ?? 12) * 15).round(),
      'line': ((style?.style.height ?? 1.4) * 240).round(),
      'lineRule': 'auto',
    },
  };
}

Map<String, dynamic> paragraphProperties(dynamic format) => {
  if (format is Map)
    ...Map<String, dynamic>.from(format['inherited'] as Map? ?? {}),
  if (format is Map)
    ...Map<String, dynamic>.from(format['direct'] as Map? ?? {}),
};
Map<String, dynamic> lineParagraphProperties(q.Line line) =>
    paragraphProperties(
      listMetadata(line.style.attributes['token']?.value)['paragraphFormat'],
    );
bool hasParagraphFormat(q.Line line) =>
    listMetadata(line.style.attributes['token']?.value)['paragraphFormat'] !=
    null;
bool _managedLine(q.Line line) =>
    hasParagraphFormat(line) ||
    !line.style.attributes.keys.any(
      (key) => key == 'code-block' || key == 'blockquote',
    );
bool _managedBlock(q.Block block) =>
    block.children.whereType<q.Line>().any(_managedLine);
q.VerticalSpacing paragraphBlockSpacing(
  BuildContext context,
  q.Block block,
  q.VerticalSpacing fallback,
) => _managedBlock(block) ? const q.VerticalSpacing(0, 0) : fallback;

q.Line? _previousLine(q.Line line) {
  q.Node? cursor = line;
  while (cursor != null) {
    final previous = cursor.previous;
    if (previous is q.Line) return previous;
    if (previous is q.Block) {
      return previous.children.whereType<q.Line>().lastOrNull;
    }
    cursor = cursor.parent;
  }
  return null;
}

q.VerticalSpacing _defaultSpacing(BuildContext context, q.Line line) {
  final styles = q.QuillStyles.getStyles(context, false);
  final h = line.style.attributes['header']?.value;
  return (switch (h) {
        1 => styles?.h1,
        2 => styles?.h2,
        3 => styles?.h3,
        4 => styles?.h4,
        5 => styles?.h5,
        6 => styles?.h6,
        _ => styles?.paragraph,
      })?.verticalSpacing ??
      const q.VerticalSpacing(0, 12);
}

double _points(Map p, String key, double fallback) =>
    p[key] is num ? math.max(0, (p[key] as num).toDouble() / 15) : fallback;
EdgeInsets paragraphPadding(
  BuildContext context,
  q.Line line,
  EdgeInsets base,
) {
  final managed =
      _managedLine(line) ||
      line.parent is q.Block && _managedBlock(line.parent! as q.Block);
  final previous = _previousLine(line);
  if (!managed && (previous == null || !hasParagraphFormat(previous))) {
    return base;
  }
  final p = lineParagraphProperties(line);
  final defaults = _defaultSpacing(context, line);
  final before = _points(p, 'before', managed ? defaults.top : base.top);
  final after = _points(p, 'after', managed ? defaults.bottom : base.bottom);
  var top = before;
  if (previous != null) {
    final old = lineParagraphProperties(previous);
    final oldAfter = _points(
      old,
      'after',
      _defaultSpacing(context, previous).bottom,
    );
    top = math.max(0, before - oldAfter);
  }
  final left = _points(p, p.containsKey('start') ? 'start' : 'left', base.left);
  final right = _points(p, p.containsKey('end') ? 'end' : 'right', base.right);
  return EdgeInsets.fromLTRB(left, top, right, after);
}

TextStyle paragraphTextStyle(
  BuildContext context,
  q.Line line,
  TextStyle base,
) => paragraphStyle(
  lineParagraphProperties(line),
  inheritedRunStyle(
    lineRunProperties(line),
    listMetadata(
              line.style.attributes['token']?.value,
            )['paragraphFormat']?['inheritedRun']
            is Map
        ? (q.QuillStyles.getStyles(context, false)?.paragraph?.style ?? base)
        : base,
  ),
);
TextStyle paragraphStyle(Map p, TextStyle base) {
  final value = p['line'];
  if (value is! num || value <= 0) return base;
  if (p['lineRule'] == null || p['lineRule'] == 'auto') {
    return base.copyWith(height: value / 240);
  }
  return base.copyWith(height: 1);
}

StrutStyle paragraphStrut(BuildContext context, q.Line line, StrutStyle base) =>
    paragraphStrutStyle(lineParagraphProperties(line), base);
StrutStyle paragraphStrutStyle(Map p, StrutStyle base) {
  final line = p['line'];
  if (line is! num ||
      line <= 0 ||
      !['exact', 'atLeast'].contains(p['lineRule'])) {
    return base;
  }
  return StrutStyle(
    fontFamily: base.fontFamily,
    fontFamilyFallback: base.fontFamilyFallback,
    fontSize: line / 15,
    height: 1,
    leading: 0,
    forceStrutHeight: p['lineRule'] == 'exact',
  );
}

List<String> paragraphPreviewLimits(Map p) {
  final limits = <String>[];
  if (['firstLine', 'hanging'].any((k) => (p[k] as num? ?? 0) != 0)) {
    limits.add('First-line or hanging indents');
  }
  if (p.keys.any((k) => k.endsWith('Chars') && (p[k] as num? ?? 0) != 0)) {
    limits.add('Character-based indents');
  }
  if (['beforeLines', 'afterLines'].any((k) => (p[k] as num? ?? 0) != 0)) {
    limits.add('Line-based paragraph spacing');
  }
  if (['beforeAutospacing', 'afterAutospacing'].any((k) => p[k] == true)) {
    limits.add('Automatic paragraph spacing');
  }
  if (['left', 'right', 'start', 'end'].any((k) => (p[k] as num? ?? 0) < 0)) {
    limits.add('Negative indents');
  }
  if ([
    'bidi',
    'mirrorIndents',
    'contextualSpacing',
    'snapToGrid',
  ].any((k) => p[k] == true)) {
    limits.add('Direction, context or document-grid settings');
  }
  return limits;
}

Delta changeParagraphFormat(
  Delta document,
  TextSelection selection,
  Map<String, dynamic> patch, {
  q.DefaultStyles? defaults,
}) {
  final change = Delta();
  var offset = 0;
  for (final line in listLines(document)) {
    if (line.end < selection.start ||
        (selection.isCollapsed
            ? line.start > selection.start
            : line.start >= selection.end)) {
      continue;
    }
    if (document
        .slice(line.start, line.end)
        .toList()
        .any((op) => op.data is! String)) {
      throw const FormatException(
        'Select paragraphs outside the table or preserved object.',
      );
    }
    final metadata = listMetadata(line.attrs['token']);
    final old = paragraphFormatForAttrs(line.attrs, defaults: defaults);
    if (patch.containsKey('outlineLevel') && old['styleId'] == null) {
      // Preserve the appearance of native text before it has a named catalog.
      final level = line.attrs['header'] as int?;
      final native = nativeParagraphStyle(level, defaults: defaults).style;
      old['styleId'] = level == null ? 'Normal' : 'Heading$level';
      if (level != null) old['inheritedOutlineLevel'] = level - 1;
      old['inheritedRun'] = {
        'sz': ((native.fontSize ?? 16) * 1.5).round(),
        'bold': native.fontWeight == FontWeight.bold,
        'italic': native.fontStyle == FontStyle.italic,
        'rFonts': {'ascii': native.fontFamily ?? 'Aptos'},
      };
    }
    final effective = paragraphProperties(old);
    final direct = Map<String, dynamic>.from(old['direct'] as Map? ?? {});
    for (final e in patch.entries) {
      if (e.value == null) {
        direct.remove(e.key);
      } else {
        direct[e.key] = e.value;
      }
    }
    // Word's start/end attributes take precedence over left/right. Resolve this
    // for each paragraph; a mixed selection can contain either representation.
    for (final pair in [('left', 'start'), ('right', 'end')]) {
      if (patch.containsKey(pair.$1) && effective.containsKey(pair.$2)) {
        if (patch[pair.$1] == null) {
          direct.remove(pair.$2);
        } else {
          direct[pair.$2] = patch[pair.$1];
        }
      }
    }
    final next = {...old, 'direct': direct};
    if (mapEquals(paragraphProperties(next), effective) &&
        patch.values.every((v) => v != null)) {
      continue;
    }
    metadata['paragraphFormat'] = next;
    change.retain(line.end - offset);
    final outline = paragraphOutlineLevel(next);
    change.retain(1, {
      'token': jsonEncode(metadata),
      if (patch.containsKey('outlineLevel'))
        'header': outline != null && outline < 9 ? outline + 1 : null,
    });
    offset = line.end + 1;
  }
  return change..trim();
}
