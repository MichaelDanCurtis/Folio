// ignore_for_file: experimental_member_use
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_quill/src/editor_toolbar_controller_shared/clipboard/clipboard_service.dart';
import 'package:flutter_quill/src/editor_toolbar_controller_shared/clipboard/clipboard_service_provider.dart';
import 'package:flutter_test/flutter_test.dart';

class _Clipboard extends ClipboardService {
  String? html;
  @override
  Future<String?> getHtmlText() async => html;
  @override
  Future<String?> getHtmlFile() async => null;
  @override
  Future<String?> getMarkdownFile() async => null;
  @override
  Future<Uint8List?> getImageFile() async => null;
  @override
  Future<Uint8List?> getGifFile() async => null;
  @override
  Future<void> copyImage(Uint8List bytes) async {}
}

void main() {
  for (final reversed in [false, true]) {
    for (final kind in ['plain', 'internal', 'html']) {
      testWidgets(
        '$kind replacement keeps caret at pasted text (${reversed ? 'reversed' : 'forward'})',
        (tester) async {
          final clipboard = _Clipboard();
          final previous = ClipboardServiceProvider.instance;
          ClipboardServiceProvider.setInstance(clipboard);
          String? plain;
          tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
            SystemChannels.platform,
            (call) async {
              if (call.method == 'Clipboard.setData') {
                plain = call.arguments['text'];
              }
              if (call.method == 'Clipboard.getData') return {'text': plain};
              if (call.method == 'Clipboard.hasStrings') {
                return {'value': plain?.isNotEmpty == true};
              }
              return null;
            },
          );
          addTearDown(() {
            ClipboardServiceProvider.setInstance(previous);
            tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
              SystemChannels.platform,
              null,
            );
          });
          q.QuillController controller(List<Map<String, dynamic>> data) {
            final value = q.QuillController(
              document: q.Document.fromJson(data),
              selection: const TextSelection.collapsed(offset: 0),
            );
            addTearDown(value.dispose);
            return value;
          }

          final target = controller([
            {'insert': 'Before\nOld text\nAfter\n'},
          ]);
          var end = 15;
          String inserted;
          if (kind == 'internal') {
            final source = controller([
              {
                'insert': 'Rich',
                'attributes': {'bold': true},
              },
              {'insert': '\n'},
            ]);
            source.updateSelection(
              const TextSelection(baseOffset: 0, extentOffset: 4),
              q.ChangeSource.local,
            );
            expect(source.clipboardSelection(true), true);
            await tester.pump();
            inserted = 'Rich';
          } else if (kind == 'html') {
            clipboard.html = '<p><em>Rich</em></p>';
            inserted = 'Rich\n';
            end = 16;
          } else {
            plain = '🚀 New';
            inserted = plain!;
          }
          target.updateSelection(
            TextSelection(
              baseOffset: reversed ? end : 7,
              extentOffset: reversed ? 7 : end,
            ),
            q.ChangeSource.local,
          );
          expect(await target.clipboardPaste(), true);
          final expected = 'Before\nOld text\nAfter\n'.replaceRange(
            7,
            end,
            inserted,
          );
          expect(target.document.toPlainText(), expected);
          expect(
            target.selection,
            TextSelection.collapsed(offset: 7 + inserted.length),
          );
          if (kind != 'plain') {
            expect(
              target.document
                  .collectStyle(7, 4)
                  .attributes[kind == 'internal' ? 'bold' : 'italic']
                  ?.value,
              true,
            );
          }
          // A following keystroke must not land in the untouched next paragraph.
          final caret = target.selection.extentOffset;
          target.replaceText(
            caret,
            0,
            '!',
            TextSelection.collapsed(offset: caret + 1),
          );
          expect(
            target.document.toPlainText(),
            expected.replaceRange(caret, caret, '!'),
          );
          expect(target.document.toPlainText(), endsWith('After\n'));
        },
      );
    }
  }
}
