import 'dart:convert';
import 'document_lists.dart';
import 'package:dart_quill_delta/dart_quill_delta.dart';

String nodeText(Map node) => node['type'] == 'hardBreak'
    ? '\n'
    : node['text'] is String
    ? node['text'] as String
    : ((node['content'] as List?) ?? [])
          .map((n) => nodeText(n as Map))
          .join(node['type'] == 'doc' || node['type'] == 'table' ? '\n' : '');

// Count projected document text, including embedded tables. Inline formatting
// must not split a word, while adjacent blocks/cells need a word boundary.
int documentWordCount(Map document) {
  String text(Map node) {
    if (node['text'] is String) return node['text'] as String;
    if (node['type'] == 'hardBreak') return '\n';
    final inline = ['paragraph', 'heading'].contains(node['type']);
    return (node['content'] as List? ?? [])
        .map((child) => text(child as Map))
        .join(inline ? '' : '\n');
  }

  return RegExp(r'\S+').allMatches(text(document)).length;
}

const _marks = {
  'bold': 'bold',
  'italic': 'italic',
  'underline': 'underline',
  'strike': 'strike',
  'code': 'code',
  'link': 'link',
};
// Unsupported blocks are opaque embeds: original JSON survives editing elsewhere.
Delta documentToDelta(Map document) {
  final delta = Delta();
  void embed(Map node) {
    delta.insert({'folioNode': jsonEncode(node)});
    delta.insert('\n');
  }

  bool simple(Map node) =>
      ((node['attrs'] as Map?) ?? {}).keys.every(
        (k) => [
          'level',
          'officeId',
          'textAlign',
          'numbering',
          'tabs',
          'paragraphFormat',
        ].contains(k),
      ) &&
      editableNumbering(node['attrs']?['numbering']) &&
      ((node['content'] as List?) ?? []).every(
        (n) =>
            (['text', 'pageField', 'image', 'noteRef'].contains(n['type']) ||
                (n['type'] == 'hardBreak' &&
                    ((n['attrs'] as Map?) ?? {}).entries.every(
                      (e) =>
                          (e.key == 'clear' && e.value == 'none') ||
                          (e.key == 'breakType' && e.value == 'textWrapping'),
                    ))) &&
            ((n['marks'] as List?) ?? []).every(
              (m) =>
                  (_marks.containsKey(m['type']) ||
                      ['textStyle', 'highlight'].contains(m['type'])) &&
                  ((m['attrs'] as Map?) ?? {}).keys.every(
                    (k) =>
                        (m['type'] == 'link' && k == 'href') ||
                        ([
                              'bold',
                              'italic',
                              'underline',
                              'strike',
                            ].contains(m['type']) &&
                            k == 'value' &&
                            m['attrs']['value'] is bool) ||
                        (m['type'] == 'textStyle' &&
                            ['fontFamily', 'fontSize', 'color'].contains(k)) ||
                        (m['type'] == 'highlight' && k == 'color'),
                  ),
            ),
      );
  void paragraph(Map node, Map<String, dynamic> block) {
    if (!simple(node)) {
      embed(node);
      return;
    }
    for (final child in (node['content'] as List?) ?? []) {
      final attrs = <String, dynamic>{};
      for (final mark in (child['marks'] as List?) ?? []) {
        if (mark['type'] == 'textStyle') {
          final a = mark['attrs'] as Map;
          for (final pair in {
            'fontFamily': 'font',
            'fontSize': 'size',
            'color': 'color',
          }.entries) {
            if (a[pair.key] != null) attrs[pair.value] = a[pair.key];
          }
          continue;
        }
        if (mark['type'] == 'highlight') {
          attrs['background'] = mark['attrs']['color'];
          continue;
        }
        attrs[_marks[mark['type']]!] = mark['type'] == 'link'
            ? mark['attrs']['href']
            : mark['attrs']?['value'] != false;
      }
      if (child['type'] == 'hardBreak') {
        // An inline atom retains source metadata and cannot be confused with
        // a literal Unicode line separator in an imported text run.
        delta.insert({
          'lineBreak': jsonEncode(child),
        }, attrs.isEmpty ? null : attrs);
      } else if (child['type'] == 'pageField') {
        delta.insert({
          'folioPageField': jsonEncode(child),
        }, attrs.isEmpty ? null : attrs);
      } else if (child['type'] == 'image') {
        // Inline pictures (W08.01): one embed character; marks stay on the run.
        delta.insert({
          'folioImage': jsonEncode(child),
        }, attrs.isEmpty ? null : attrs);
      } else if (child['type'] == 'noteRef') {
        // Footnote and endnote references (W11.01).
        delta.insert({
          'folioNote': jsonEncode(child),
        }, attrs.isEmpty ? null : attrs);
      } else if ((child['text'] as String).isNotEmpty) {
        delta.insert(child['text'], attrs.isEmpty ? null : attrs);
      }
    }
    final a = node['attrs'] as Map? ?? {};
    final metadata = {
      if (a['officeId'] != null) 'officeId': a['officeId'],
      if (a.containsKey('numbering')) 'numbering': a['numbering'],
      if (a.containsKey('tabs')) 'tabs': a['tabs'],
      if (a.containsKey('paragraphFormat'))
        'paragraphFormat': a['paragraphFormat'],
    };
    if (metadata.isNotEmpty) block['token'] = jsonEncode(metadata);
    if (a['numbering'] is Map) {
      final n = a['numbering'] as Map;
      block['list'] = n['levels'][n['level']]['format'] == 'bullet'
          ? 'bullet'
          : 'ordered';
      if (n['level'] > 0) block['indent'] = n['level'];
    }
    if (a['textAlign'] != null) block['align'] = a['textAlign'];
    delta.insert('\n', block.isEmpty ? null : block);
  }

  bool isList(Map n) => ['bulletList', 'orderedList'].contains(n['type']);
  bool simpleList(Map n) =>
      ((n['attrs'] as Map?) ?? {}).keys.every((k) => k == 'level') &&
      (n['content'] as List? ?? []).isNotEmpty &&
      (n['content'] as List).every((item) {
        final children = item['content'] as List? ?? [];
        return (item['attrs'] as Map? ?? {}).isEmpty &&
            children.isNotEmpty &&
            ['paragraph', 'heading'].contains(children.first['type']) &&
            simple(children.first) &&
            children.skip(1).every((c) => isList(c) && simpleList(c));
      });
  void writeList(Map n, int depth) {
    final level = n['attrs']?['level'] as int? ?? depth;
    for (final item in n['content']) {
      paragraph(item['content'][0], {
        if (item['content'][0]['type'] == 'heading')
          'header': item['content'][0]['attrs']?['level'] ?? 1,
        'list': n['type'] == 'bulletList' ? 'bullet' : 'ordered',
        if (level > 0) 'indent': level,
      });
      for (final nested in (item['content'] as List).skip(1)) {
        writeList(nested, level + 1);
      }
    }
  }

  for (final raw in (document['content'] as List?) ?? []) {
    final node = raw as Map;
    switch (node['type']) {
      case 'paragraph':
        paragraph(node, {});
      case 'heading':
        paragraph(node, {'header': node['attrs']?['level'] ?? 1});
      case 'bulletList':
      case 'orderedList':
        if (simpleList(node)) {
          writeList(node, 0);
        } else {
          embed(node);
        }
      default:
        embed(node);
    }
  }
  if (delta.isEmpty) delta.insert('\n');
  return delta;
}

Map<String, dynamic> deltaToDocument(Delta delta, {Map? attributes}) {
  final nodes = <Map<String, dynamic>>[];
  var inline = <Map<String, dynamic>>[];
  bool embeddedLine = false;
  final lists = <({int level, Map<String, dynamic> node})>[];
  void endLine(Map attrs) {
    if (embeddedLine) {
      embeddedLine = false;
      return;
    }
    final metadata = <String, dynamic>{};
    if (attrs['token'] is String) {
      try {
        metadata.addAll(Map<String, dynamic>.from(jsonDecode(attrs['token'])));
      } catch (_) {}
    }
    if (metadata.containsKey('numbering')) {
      if (metadata['numbering'] == null && attrs['list'] != null) {
        metadata.remove('numbering');
      } else {
        metadata['numbering'] = numberingForLine(attrs);
      }
    }
    if (attrs['header'] != null) metadata['level'] = attrs['header'];
    if (attrs['align'] != null) metadata['textAlign'] = attrs['align'];
    final paragraph = <String, dynamic>{
      'type': attrs['header'] != null ? 'heading' : 'paragraph',
      if (metadata.isNotEmpty) 'attrs': metadata,
      if (inline.isNotEmpty) 'content': inline,
    };
    inline = [];
    if (attrs['list'] != null && !metadata.containsKey('numbering')) {
      final type = attrs['list'] == 'ordered' ? 'orderedList' : 'bulletList';
      final level = ((attrs['indent'] as num?)?.toInt() ?? 0).clamp(0, 8);
      while (lists.isNotEmpty && lists.last.level > level) {
        lists.removeLast();
      }
      if (lists.isNotEmpty &&
          lists.last.level == level &&
          lists.last.node['type'] != type) {
        lists.removeLast();
      }
      if (lists.isEmpty || lists.last.level < level) {
        final expected = lists.isEmpty ? 0 : lists.last.level + 1;
        final list = <String, dynamic>{
          'type': type,
          if (level != expected) 'attrs': {'level': level},
          'content': <Map<String, dynamic>>[],
        };
        if (lists.isEmpty) {
          nodes.add(list);
        } else {
          (((lists.last.node['content'] as List).last['content']) as List).add(
            list,
          );
        }
        lists.add((level: level, node: list));
      }
      (lists.last.node['content'] as List).add({
        'type': 'listItem',
        'content': [paragraph],
      });
    } else {
      lists.clear();
      nodes.add(paragraph);
    }
  }

  for (final op in delta.toJson()) {
    final insert = op['insert'];
    final attrs = (op['attributes'] as Map?) ?? {};
    if (insert is Map &&
        (insert['folioPageField'] is String ||
            insert['lineBreak'] is String ||
            insert['folioImage'] is String ||
            insert['folioNote'] is String)) {
      final field = Map<String, dynamic>.from(
        jsonDecode(
          insert['folioPageField'] ??
              insert['lineBreak'] ??
              insert['folioImage'] ??
              insert['folioNote'],
        ),
      );
      final marks = _readMarks(attrs);
      field.remove('marks');
      if (marks.isNotEmpty) field['marks'] = marks;
      inline.add(field);
      continue;
    }
    if (insert is Map) {
      lists.clear();
      if (inline.isNotEmpty) endLine({});
      if (insert['folioNode'] is String) {
        nodes.add(Map<String, dynamic>.from(jsonDecode(insert['folioNode'])));
      } else {
        nodes.add({
          'type': 'paragraph',
          'content': [
            {'type': 'text', 'text': '[Unsupported pasted media]'},
          ],
        });
      }
      embeddedLine = true;
      continue;
    }
    if (insert is! String) continue;
    final parts = insert.split('\n');
    for (var i = 0; i < parts.length; i++) {
      if (parts[i].isNotEmpty) {
        final marks = _readMarks(attrs);
        inline.add({
          'type': 'text',
          'text': parts[i],
          if (marks.isNotEmpty) 'marks': marks,
        });
      }
      if (i < parts.length - 1) endLine(attrs);
    }
  }
  if (inline.isNotEmpty) endLine({});
  return {
    'type': 'doc',
    if (attributes != null) 'attrs': Map<String, dynamic>.from(attributes),
    'content': nodes.isEmpty
        ? [
            {'type': 'paragraph'},
          ]
        : nodes,
  };
}

List<Map<String, dynamic>> _readMarks(Map attrs) {
  final marks = <Map<String, dynamic>>[];
  for (final entry in _marks.entries) {
    if (attrs[entry.value] != null) {
      marks.add({
        'type': entry.key,
        if (entry.key == 'link') 'attrs': {'href': attrs['link']},
        if (attrs[entry.value] == false) 'attrs': {'value': false},
      });
    }
  }
  final textStyle = <String, dynamic>{};
  for (final pair in {
    'font': 'fontFamily',
    'size': 'fontSize',
    'color': 'color',
  }.entries) {
    if (attrs[pair.key] != null) textStyle[pair.value] = attrs[pair.key];
  }
  if (textStyle.isNotEmpty) {
    marks.add({'type': 'textStyle', 'attrs': textStyle});
  }
  if (attrs['background'] != null) {
    marks.add({
      'type': 'highlight',
      'attrs': {'color': attrs['background']},
    });
  }
  return marks;
}
