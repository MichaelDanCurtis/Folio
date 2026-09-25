import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_case.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  test('each case matches Word', () {
    const text = "hELLO wORLD. it's 3rd-rate! ok?  sTRASSE";
    expect(
      changeCase(text, TextCase.sentence),
      "Hello world. It's 3rd-rate! Ok?  Strasse",
    );
    expect(
      changeCase(text, TextCase.lower),
      "hello world. it's 3rd-rate! ok?  strasse",
    );
    expect(
      changeCase(text, TextCase.upper),
      "HELLO WORLD. IT'S 3RD-RATE! OK?  STRASSE",
    );
    expect(
      changeCase(text, TextCase.capitalize),
      "Hello World. It's 3rd-Rate! Ok?  Strasse",
    );
    expect(
      changeCase(text, TextCase.toggle),
      "Hello World. IT'S 3RD-RATE! OK?  Strasse",
    );
    expect(changeCase('one.\ntwo', TextCase.sentence), 'One.\nTwo');
  });

  test('Shift+F3 cycles UPPERCASE, lowercase, Capitalize Each Word', () {
    expect(nextCaseCycle('Mixed words'), TextCase.upper);
    expect(nextCaseCycle('MIXED WORDS'), TextCase.lower);
    expect(nextCaseCycle('mixed words'), TextCase.capitalize);
    expect(nextCaseCycle('Mixed Words'), TextCase.upper);
  });

  test('case changes keep each run and paragraph mark', () {
    final doc = Delta()
      ..insert('big ', {'bold': true})
      ..insert('strasse', {'link': 'https://example.org'})
      ..insert('\n', {'header': 1})
      ..insert('tail\n');
    final change = changeCaseDelta(doc, 0, 16, TextCase.upper);
    expect(doc.compose(change).toJson(), [
      {
        'insert': 'BIG ',
        'attributes': {'bold': true},
      },
      {
        'insert': 'STRASSE',
        'attributes': {'link': 'https://example.org'},
      },
      {
        'insert': '\n',
        'attributes': {'header': 1},
      },
      {'insert': 'TAIL\n'},
    ]);
  });

  test('Clear All Formatting keeps links, lists and Office identity', () {
    final token = jsonEncode({
      'officeId': 'p1',
      'tabs': {'direct': []},
      'paragraphFormat': {
        'direct': {'left': 720},
      },
    });
    final doc = Delta()
      ..insert('Loud', {'bold': true, 'color': '#ff0000', 'size': '20'})
      ..insert(' link', {'link': 'https://example.org', 'italic': true})
      ..insert('\n', {'header': 2, 'align': 'center', 'token': token})
      ..insert('Item')
      ..insert('\n', {'list': 'bullet'});
    final change = clearFormattingDelta(
      doc,
      const TextSelection(baseOffset: 0, extentOffset: 13),
    );
    final after = doc.compose(change).toJson();
    expect(after, [
      {'insert': 'Loud'},
      {
        'insert': ' link',
        'attributes': {'link': 'https://example.org'},
      },
      {
        'insert': '\n',
        'attributes': {
          'token': jsonEncode({'officeId': 'p1'}),
        },
      },
      {'insert': 'Item'},
      {
        'insert': '\n',
        'attributes': {'list': 'bullet'},
      },
    ]);
  });

  test('Clear All Formatting applies the catalog Normal style', () {
    final doc = Delta()
      ..insert('Title')
      ..insert('\n', {'header': 1});
    final change = clearFormattingDelta(
      doc,
      const TextSelection.collapsed(offset: 2),
      styles: {
        'defaultStyle': 'Normal',
        'styles': [
          {
            'id': 'Normal',
            'name': 'Normal',
            'type': 'paragraph',
            'properties': {
              'paragraph': {'after': 160},
              'tabs': [],
              'run': {'sz': 22},
            },
          },
        ],
      },
    );
    final line = doc.compose(change).toList().last;
    expect(line.attributes!.containsKey('header'), false);
    final format = jsonDecode(line.attributes!['token'])['paragraphFormat'];
    expect(format['styleId'], 'Normal');
    expect(format['inheritedRun'], {'sz': 22});
    expect(format['inherited']['after'], 160);
  });

  testWidgets('toolbar and Shift+F3 change case in one Undo step', (t) async {
    final controller = q.QuillController(
      document: q.Document.fromDelta(
        Delta()
          ..insert('quiet words', {'italic': true})
          ..insert('\n'),
      ),
      selection: const TextSelection(baseOffset: 0, extentOffset: 11),
    );
    addTearDown(controller.dispose);
    await t.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
        home: Scaffold(
          body: DocumentView(controller: controller, undo: () {}, redo: () {}),
        ),
      ),
    );
    await t.pumpAndSettle();
    t
        .widget<q.QuillEditor>(find.byType(q.QuillEditor))
        .focusNode
        .requestFocus();
    controller.updateSelection(
      const TextSelection(baseOffset: 0, extentOffset: 11),
      q.ChangeSource.local,
    );
    await t.pump();
    String text() => controller.document.toPlainText();
    for (final expected in ['Quiet Words', 'QUIET WORDS', 'quiet words']) {
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await t.sendKeyEvent(LogicalKeyboardKey.f3);
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await t.pump();
      expect(text(), '$expected\n');
      expect(controller.selection.end, 11);
    }
    expect(controller.document.toDelta().first.attributes, {'italic': true});

    await t.tap(find.byKey(const Key('document-change-case')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('change-case-sentence')));
    await t.pumpAndSettle();
    expect(text(), 'Quiet words\n');
    controller.undo();
    expect(text(), 'quiet words\n');

    controller.updateSelection(
      const TextSelection(baseOffset: 0, extentOffset: 11),
      q.ChangeSource.local,
    );
    await t.tap(find.byKey(const Key('document-clear-formatting')));
    await t.pump();
    expect(controller.document.toDelta().first.attributes, isNull);
    controller.undo();
    expect(controller.document.toDelta().first.attributes, {'italic': true});
  });
}
