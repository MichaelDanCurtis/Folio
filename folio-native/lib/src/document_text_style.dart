import 'document_case.dart';
import 'document_fields.dart';
import 'document_line_breaks.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_lists.dart';

Map<String, dynamic> lineRunProperties(q.Line line) =>
    Map<String, dynamic>.from(
      listMetadata(
                line.style.attributes['token']?.value,
              )['paragraphFormat']?['inheritedRun']
              as Map? ??
          {},
    );
Color? wordTextColor(dynamic value) {
  if (value is! String) return null;
  final hex = value.replaceFirst('#', '');
  if (!RegExp(r'^[0-9a-fA-F]{6}$').hasMatch(hex)) return null;
  return Color(0xff000000 | int.parse(hex, radix: 16));
}

TextStyle inheritedRunStyle(Map run, TextStyle base) {
  final size = run['sz'];
  final font = run['rFonts'] as Map? ?? {};
  final underline = run['u'] != null && run['u'] != 'none';
  return base.copyWith(
    fontFamily: font['ascii'] as String? ?? font['hAnsi'] as String?,
    fontSize: size is num && size > 0 ? size / 2 * 4 / 3 : null,
    fontWeight: run['bold'] is bool
        ? (run['bold'] == true ? FontWeight.bold : FontWeight.normal)
        : null,
    fontStyle: run['italic'] is bool
        ? (run['italic'] == true ? FontStyle.italic : FontStyle.normal)
        : null,
    color: wordTextColor(run['color']?['val']),
    decoration: TextDecoration.combine([
      if (underline) TextDecoration.underline,
      if (run['strike'] == true) TextDecoration.lineThrough,
    ]),
    decorationStyle: switch (run['u']) {
      'double' => TextDecorationStyle.double,
      'wave' || 'wavyHeavy' || 'wavyDouble' => TextDecorationStyle.wavy,
      'dotted' || 'dottedHeavy' => TextDecorationStyle.dotted,
      'dash' ||
      'dashLong' ||
      'dashedHeavy' ||
      'dashLongHeavy' => TextDecorationStyle.dashed,
      _ => TextDecorationStyle.solid,
    },
  );
}

TextStyle directRunStyle(Map run, Map attrs, TextStyle base) {
  final inheritedUnderline = run['u'] != null && run['u'] != 'none';
  final underline = attrs.containsKey('underline')
      ? attrs['underline'] == true
      : attrs['link'] != null ||
            inheritedUnderline ||
            (base.decoration?.contains(TextDecoration.underline) ?? false);
  final strike = attrs.containsKey('strike')
      ? attrs['strike'] == true
      : run['strike'] == true ||
            (base.decoration?.contains(TextDecoration.lineThrough) ?? false);
  final size = double.tryParse('${attrs['size']}');
  return base.copyWith(
    fontWeight: attrs['bold'] is bool
        ? (attrs['bold'] == true ? FontWeight.bold : FontWeight.normal)
        : null,
    fontStyle: attrs['italic'] is bool
        ? (attrs['italic'] == true ? FontStyle.italic : FontStyle.normal)
        : null,
    fontSize: size != null && size > 0 ? size * 4 / 3 : null,
    decoration: TextDecoration.combine([
      if (underline) TextDecoration.underline,
      if (strike) TextDecoration.lineThrough,
    ]),
  );
}

TextStyle paragraphInlineStyle(
  BuildContext context,
  q.Line line,
  Map<String, dynamic> attrs,
  TextStyle base,
) => directRunStyle(lineRunProperties(line), attrs, base);
TextAlign paragraphAlignment(
  BuildContext context,
  q.Line line,
  TextAlign fallback,
) => switch (listMetadata(
  line.style.attributes['token']?.value,
)['paragraphFormat']?['inheritedAlignment']) {
  'center' => TextAlign.center,
  'right' || 'end' => TextAlign.end,
  'left' || 'start' => TextAlign.start,
  'both' || 'distribute' => TextAlign.justify,
  _ => fallback,
};

/// Shift+F3 cycles the case of the selection (W01.13).
class CycleTextCase extends Intent {
  const CycleTextCase();
}

class ToggleDocumentMark extends Intent {
  final String mark;
  const ToggleDocumentMark(this.mark);
}

const documentMarkShortcuts = <ShortcutActivator, Intent>{
  ...documentLineBreakShortcuts,
  SingleActivator(LogicalKeyboardKey.keyB, meta: true): ToggleDocumentMark(
    'bold',
  ),
  SingleActivator(LogicalKeyboardKey.keyB, control: true): ToggleDocumentMark(
    'bold',
  ),
  SingleActivator(LogicalKeyboardKey.keyI, meta: true): ToggleDocumentMark(
    'italic',
  ),
  SingleActivator(LogicalKeyboardKey.keyI, control: true): ToggleDocumentMark(
    'italic',
  ),
  SingleActivator(LogicalKeyboardKey.keyU, meta: true): ToggleDocumentMark(
    'underline',
  ),
  SingleActivator(LogicalKeyboardKey.keyU, control: true): ToggleDocumentMark(
    'underline',
  ),
  SingleActivator(LogicalKeyboardKey.f3, shift: true): CycleTextCase(),
};
bool _inheritedMark(Map run, String key) => key == 'underline'
    ? run['u'] != null && run['u'] != 'none'
    : run[key] == true;
bool documentMarkEnabled(q.QuillController controller, String key) {
  if (controller.selection.isCollapsed) {
    final direct = controller.getSelectionStyle().attributes[key]?.value;
    if (direct is bool) return direct;
  }
  final delta = controller.document.toDelta();
  final selection = controller.selection;
  final values = <bool>[];
  for (final line in listLines(delta)) {
    if (line.end < selection.start ||
        (selection.isCollapsed
            ? line.start > selection.start
            : line.start >= selection.end)) {
      continue;
    }
    final run =
        listMetadata(line.attrs['token'])['paragraphFormat']?['inheritedRun']
            as Map? ??
        {};
    if (selection.isCollapsed) {
      values.add(_inheritedMark(run, key));
      continue;
    }
    final start = selection.start.clamp(line.start, line.end),
        end = selection.end.clamp(line.start, line.end);
    for (final op in delta.slice(start, end).toList()) {
      if (op.data is String) {
        values.add(
          op.attributes?[key] is bool
              ? op.attributes![key] == true
              : _inheritedMark(run, key),
        );
      }
    }
    if (start == end) values.add(_inheritedMark(run, key));
  }
  return values.isNotEmpty && values.every((v) => v);
}

void toggleDocumentMark(q.QuillController controller, String key) {
  controller.formatSelection(
    q.Attribute.fromKeyValue(key, !documentMarkEnabled(controller, key))!,
  );
}

Map<Type, Action<Intent>> documentMarkActions(
  q.QuillController controller,
  FocusNode focus,
) => {
  ...documentLineBreakActions(controller),
  ToggleDocumentMark: CallbackAction<ToggleDocumentMark>(
    onInvoke: (intent) {
      toggleDocumentMark(controller, intent.mark);
      focus.requestFocus();
      return null;
    },
  ),
  CycleTextCase: CallbackAction<CycleTextCase>(
    onInvoke: (_) {
      applyTextCase(controller);
      focus.requestFocus();
      return null;
    },
  ),
};

class DocumentTextToolbar extends StatelessWidget {
  final q.QuillController controller;
  final FocusNode focus;

  /// The document's style catalog, for Clear All Formatting's Normal style.
  final Map? Function()? styles;
  const DocumentTextToolbar({
    required this.controller,
    required this.focus,
    this.styles,
    super.key,
  });
  @override
  Widget build(BuildContext context) => Row(
    children: [
      AnimatedBuilder(
        animation: controller,
        builder: (context, _) => Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            for (final item in const [
              ('bold', 'Bold', Icons.format_bold),
              ('italic', 'Italic', Icons.format_italic),
              ('underline', 'Underline', Icons.format_underline),
              ('strike', 'Strikethrough', Icons.format_strikethrough),
            ])
              IconButton(
                key: ValueKey('document-mark-${item.$1}'),
                tooltip: item.$2,
                isSelected: documentMarkEnabled(controller, item.$1),
                icon: Icon(item.$3, size: 19),
                visualDensity: VisualDensity.compact,
                onPressed: () {
                  toggleDocumentMark(controller, item.$1);
                  focus.requestFocus();
                },
              ),
            PopupMenuButton<TextCase>(
              key: const Key('document-change-case'),
              tooltip: 'Change case (Shift+F3)',
              icon: const Icon(Icons.text_fields, size: 19),
              onSelected: (mode) {
                applyTextCase(controller, mode);
                focus.requestFocus();
              },
              itemBuilder: (_) => [
                for (final entry in textCaseLabels.entries)
                  PopupMenuItem(
                    key: ValueKey('change-case-${entry.key.name}'),
                    value: entry.key,
                    child: Text(entry.value),
                  ),
              ],
            ),
            IconButton(
              key: const Key('document-clear-formatting'),
              tooltip: 'Clear all formatting',
              icon: const Icon(Icons.format_clear, size: 19),
              visualDensity: VisualDensity.compact,
              onPressed: () {
                applyClearFormatting(controller, styles: styles?.call());
                focus.requestFocus();
              },
            ),
          ],
        ),
      ),
      Expanded(
        child: q.QuillSimpleToolbar(
          controller: controller,
          config: const q.QuillSimpleToolbarConfig(
            multiRowsDisplay: false,
            showUndo: false,
            showRedo: false,
            showBoldButton: false,
            showItalicButton: false,
            showUnderLineButton: false,
            showStrikeThrough: false,
            showHeaderStyle: false,
            showAlignmentButtons: true,
            showInlineCode: false,
            showCodeBlock: false,
            showListCheck: false,
            showQuote: false,
            showSubscript: false,
            showSuperscript: false,
            showClearFormat: false,
            showLink: false,
          ),
        ),
      ),
      IconButton(
        key: const Key('document-line-break'),
        tooltip: 'Line break (Shift+Enter)',
        icon: const Icon(Icons.keyboard_return, size: 19),
        onPressed: () {
          insertDocumentLineBreak(controller);
          focus.requestFocus();
        },
      ),
      PopupMenuButton<String>(
        key: const Key('insert-page-field'),
        tooltip: 'Insert page field',
        icon: const Icon(Icons.numbers, size: 19),
        onSelected: (kind) {
          insertPageField(controller, kind);
          focus.requestFocus();
        },
        itemBuilder: (_) => [
          for (final entry in pageFieldLabels.entries)
            PopupMenuItem(value: entry.key, child: Text(entry.value)),
        ],
      ),
    ],
  );
}
