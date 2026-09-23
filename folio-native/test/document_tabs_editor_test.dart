import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_tabs.dart';

void main() {
  for (final leader in ['none', 'dot']) {
    testWidgets(
      'tabbed editor preserves input composition, clipboard, cursor and undo with $leader leader',
      (tester) async {
        final controller = q.QuillController(
          document: q.Document.fromJson([
            {'insert': 'A\tB\tTail'},
            {
              'insert': '\n',
              if (leader != 'none')
                'attributes': {
                  'token': jsonEncode({
                    'tabs': {
                      'inherited': [],
                      'direct': [
                        DocumentTabStop(2160, leader: leader).toJson(),
                      ],
                    },
                  }),
                },
            },
          ]),
          selection: const TextSelection.collapsed(offset: 0),
        );
        final focus = FocusNode();
        final scroll = ScrollController();
        String? clipboard;
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          (call) async {
            if (call.method == 'Clipboard.setData') {
              clipboard = (call.arguments as Map)['text'] as String?;
            }
            if (call.method == 'Clipboard.getData') return {'text': clipboard};
            if (call.method == 'Clipboard.hasStrings') {
              return {'value': clipboard?.isNotEmpty ?? false};
            }
            return null;
          },
        );
        addTearDown(() async {
          await tester.pumpWidget(const SizedBox.shrink());
          controller.dispose();
          focus.dispose();
          scroll.dispose();
          tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
            SystemChannels.platform,
            null,
          );
        });
        await tester.pumpWidget(
          MaterialApp(
            localizationsDelegates: const [
              q.FlutterQuillLocalizations.delegate,
            ],
            builder: (context, child) => q.QuillParagraphLayout(
              builder: layoutDocumentTabs,
              child: child!,
            ),
            home: Scaffold(
              body: Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: 400,
                  child: q.QuillEditor.basic(
                    controller: controller,
                    focusNode: focus,
                    scrollController: scroll,
                    config: const q.QuillEditorConfig(scrollable: false),
                  ),
                ),
              ),
            ),
          ),
        );
        focus.requestFocus();
        await tester.pumpAndSettle();
        final raw = tester.state<q.QuillRawEditorState>(
          find.byType(q.QuillRawEditor),
        );
        final original = controller.document.toDelta().toJson();
        final caret = raw.renderEditor.getLocalRectForCaret(
          const TextPosition(offset: 2),
        );
        await tester.tapAt(
          raw.renderEditor.localToGlobal(
            Offset(caret.left - 1, caret.center.dy),
          ),
        );
        await tester.pumpAndSettle();
        expect(controller.selection.baseOffset, 2);
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pumpAndSettle();
        expect(controller.document.toPlainText(), 'A\t\tB\tTail\n');
        expect(controller.selection.baseOffset, 3);
        controller.undo();
        await tester.pumpAndSettle();
        expect(controller.document.toDelta().toJson(), original);

        final text = raw.textEditingValue.text;
        raw.updateEditingValue(
          TextEditingValue(
            text: text.replaceFirst('B', '漢'),
            selection: const TextSelection.collapsed(offset: 3),
            composing: const TextRange(start: 2, end: 3),
          ),
        );
        await tester.pumpAndSettle();
        expect(controller.document.toPlainText(), 'A\t漢\tTail\n');
        expect(tester.takeException(), isNull);
        raw.updateEditingValue(
          raw.textEditingValue.copyWith(composing: TextRange.empty),
        );
        await tester.pumpAndSettle();
        controller.updateSelection(
          const TextSelection(baseOffset: 0, extentOffset: 8),
          q.ChangeSource.local,
        );
        raw.copySelection(SelectionChangedCause.keyboard);
        await tester.pumpAndSettle();
        expect(clipboard, 'A\t漢\tTail');
        controller.undo();
        await tester.pumpAndSettle();
        expect(controller.document.toDelta().toJson(), original);
        expect(tester.takeException(), isNull);
      },
      variant: TargetPlatformVariant.only(TargetPlatform.macOS),
    );
  }
}
