import 'document_builtin_styles.dart';
import 'document_style_resolution.dart';
import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
// Quill 11.6.0 is pinned. Its built-in rule keeps token metadata on the first
// pasted line only; extend that rule for Folio's numbered paragraphs.
// ignore: implementation_imports
import 'package:flutter_quill/src/rules/insert.dart';
// ignore: implementation_imports
import 'package:flutter_quill/src/rules/format.dart';
import 'document_lists.dart';
import 'document_paragraph_format.dart';

void configureDocumentLists(
  q.Document document, {
  Map? Function()? styleCatalog,
  Set<String> Function()? sectionBoundaries,
}) => document.setCustomRules([
  NumberedParagraphInsertRule(
    styleCatalog: styleCatalog,
    sectionBoundaries: sectionBoundaries,
  ),
  NamedParagraphFormatRule(styleCatalog: styleCatalog),
]);

/// Numbering and a paragraph style can coexist in Word. Keep both when Quill
/// applies its otherwise exclusive heading/list attributes. Conventional
/// heading commands also refresh the named declaration and its inherited data.
class NamedParagraphFormatRule extends FormatRule {
  final Map? Function()? styleCatalog;
  const NamedParagraphFormatRule({this.styleCatalog});
  @override
  Delta? applyRule(
    q.Document document,
    int index, {
    int? len,
    Object? data,
    q.Attribute? attribute,
  }) {
    if (attribute?.key != 'header' && attribute?.key != 'list') return null;
    final change = const ResolveLineFormatRule().applyRule(
      document,
      index,
      len: len,
      attribute: attribute,
    );
    if (change == null) return null;
    final before = listLines(document.toDelta());
    final after = listLines(document.toDelta().compose(change));
    final catalog = expandedBuiltinStyles(styleCatalog?.call());
    final patch = Delta();
    var consumed = 0;
    for (var i = 0; i < before.length; i++) {
      final old = before[i], line = after[i];
      if (line.end < index || line.start > index + (len ?? 0)) continue;
      final attrs = <String, dynamic>{};
      if (attribute!.key == 'list' && old.attrs['header'] != null) {
        attrs['header'] = old.attrs['header'];
      }
      if (attribute.key == 'header') {
        if (old.attrs['list'] != null) attrs['list'] = old.attrs['list'];
        final target = catalog == null
            ? null
            : attribute.value == null
            ? builtinBodyStyle(catalog)
            : builtinHeadingStyle(catalog, attribute.value as int);
        if (catalog != null && target != null) {
          final metadata = listMetadata(old.attrs['token']);
          metadata['paragraphFormat'] = namedParagraphFormat(
            catalog,
            target['id'] as String,
            direct: metadata['paragraphFormat']?['direct'] as Map?,
          );
          final outline = paragraphOutlineLevel(metadata['paragraphFormat']);
          attrs['header'] = outline != null && outline < 9 ? outline + 1 : null;
          final tabs =
              resolveNamedStyle(catalog, target['id'] as String)['tabs']
                  as List;
          if (metadata['tabs'] != null || tabs.isNotEmpty) {
            metadata['tabs'] = {
              'inherited': tabs,
              'direct': metadata['tabs']?['direct'] ?? [],
            };
          }
          attrs['token'] = jsonEncode(metadata);
        }
      }
      if (attrs.isNotEmpty) {
        patch.retain(line.end - consumed);
        patch.retain(1, attrs);
        consumed = line.end + 1;
      }
    }
    return change.compose(patch);
  }
}

class NumberedParagraphInsertRule extends PreserveBlockStyleOnInsertRule {
  final Map? Function()? styleCatalog;
  final Set<String> Function()? sectionBoundaries;
  const NumberedParagraphInsertRule({
    this.styleCatalog,
    this.sectionBoundaries,
  });
  @override
  Delta? applyRule(
    q.Document document,
    int index, {
    int? len,
    Object? data,
    q.Attribute? attribute,
  }) {
    if (data is! String || !data.contains('\n')) return null;
    final source = listLines(
      document.toDelta(),
    ).where((l) => index >= l.start && index <= l.end).firstOrNull;
    if (source == null) return null;
    final sourceMetadata = listMetadata(source.attrs['token']);
    if (numberingForLine(source.attrs) == null &&
        sourceMetadata['paragraphFormat'] == null &&
        sourceMetadata['tabs'] == null) {
      return null;
    }
    final exit = const AutoExitBlockRule().applyRule(
      document,
      index,
      len: len,
      data: data,
      attribute: attribute,
    );
    if (exit != null) return exit;
    var insertion = super.applyRule(
      document,
      index,
      len: len,
      data: data,
      attribute: attribute,
    );
    // Token-only paragraphs are not Quill blocks. Still use Quill's ordinary
    // split/heading/inline rules, then carry Folio metadata to the new lines.
    for (final rule in const [
      PreserveLineStyleOnSplitRule(),
      ResetLineFormatOnNewLineRule(),
      PreserveInlineStylesRule(),
      CatchAllInsertRule(),
    ]) {
      insertion ??= rule.applyRule(
        document,
        index,
        len: len,
        data: data,
        attribute: attribute,
      );
    }
    if (insertion == null) return null;
    final next = document.toDelta().compose(insertion);
    final metadata = listMetadata(source.attrs['token']);
    final patch = Delta();
    var consumed = 0, first = true;
    for (final line in listLines(next)) {
      if (line.start < source.start || line.end > source.end + data.length) {
        continue;
      }
      final token = {...metadata};
      if (!first &&
          !(sectionBoundaries?.call().contains(metadata['officeId']) ?? false)) {
        token.remove('officeId');
      }
      final outline = paragraphOutlineLevel(token['paragraphFormat']);
      final block = <String, dynamic>{
        if (outline is int && outline < 9) 'header': outline + 1,
      };
      final catalog = expandedBuiltinStyles(styleCatalog?.call());
      if (!first &&
          data == '\n' &&
          index == source.end &&
          source.attrs['list'] == null &&
          catalog != null) {
        final id = metadata['paragraphFormat']?['styleId'];
        final styles = {
          for (final s in catalog['styles'] as List? ?? []) s['id']: s as Map,
        };
        final next = styles[id]?['next'];
        if (next is String &&
            next != id &&
            styles[next]?['type'] == 'paragraph') {
          final format = namedParagraphFormat(
            catalog,
            next,
            direct: metadata['paragraphFormat']?['direct'] as Map?,
          );
          token['paragraphFormat'] = format;
          final resolved = resolveNamedStyle(catalog, next);
          final directTabs = metadata['tabs']?['direct'] as List? ?? [];
          if ((resolved['tabs'] as List).isNotEmpty ||
              metadata['tabs'] != null) {
            token['tabs'] = {
              'inherited': resolved['tabs'],
              'direct': directTabs,
            };
          }
          final level = paragraphOutlineLevel(format);
          block['header'] = level is int && level < 9 ? level + 1 : null;
        }
      }
      patch.retain(line.end - consumed);
      patch.retain(1, {'token': jsonEncode(token), ...block});
      consumed = line.end + 1;
      first = false;
    }
    return insertion.compose(patch);
  }
}
