import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

const pageFieldLabels = {
  'PAGE': 'Page number',
  'NUMPAGES': 'Total pages',
  'SECTIONPAGES': 'Pages in section',
  'SECTION': 'Section number',
};
Map<String, dynamic> newPageField(String kind) => {
  'type': 'pageField',
  'text': '?',
  'attrs': {
    'kind': kind,
    'instruction': ' $kind ',
    'dirty': true,
    'locked': false,
  },
};
void insertPageField(q.QuillController controller, String kind) {
  if (controller.readOnly || !pageFieldLabels.containsKey(kind)) return;
  final selection = controller.selection;
  if (!selection.isValid) return;
  controller.replaceText(
    selection.start,
    selection.end - selection.start,
    q.Embeddable('folioPageField', jsonEncode(newPageField(kind))),
    TextSelection.collapsed(offset: selection.start + 1),
  );
}

class PageFieldEmbedBuilder extends q.EmbedBuilder {
  const PageFieldEmbedBuilder();
  @override
  String get key => 'folioPageField';
  @override
  bool get expanded => false;
  @override
  String toPlainText(q.Embed node) =>
      jsonDecode(node.value.data)['text'] as String;
  @override
  Widget build(BuildContext context, q.EmbedContext embedContext) {
    final field = jsonDecode(embedContext.node.value.data) as Map;
    final label = pageFieldLabels[field['attrs']['kind']] ?? 'Page field';
    final cached = field['text'];
    return Tooltip(
      message:
          '$label field. ${cached == '?' ? 'Value awaits pagination.' : 'Last saved value: $cached. Folio has not recalculated it.'}',
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
        decoration: BoxDecoration(
          color: const Color(0xffe5edf7),
          borderRadius: BorderRadius.circular(3),
        ),
        child: Text('‹$label›', style: embedContext.textStyle),
      ),
    );
  }
}
