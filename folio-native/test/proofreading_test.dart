import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/proofreading.dart';

q.QuillController editorFor(String text) => q.QuillController(
  document: q.Document()..insert(0, text),
  selection: const TextSelection.collapsed(offset: 0),
);
Map<String, dynamic> resultFor(String text) {
  final index = text.indexOf('recieved');
  return {
    'issues': [
      if (index >= 0)
        {
          'start': index,
          'end': index + 8,
          'text': 'recieved',
          'category': 'spelling',
          'message': 'Check spelling.',
          'suggestions': ['received'],
        },
    ],
    'truncated': false,
  };
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('unchanged styled words survive a wider grammar correction', () {
    final editor = editorFor('This is an letter.');
    addTearDown(editor.dispose);
    editor.formatText(11, 6, q.Attribute.italic);
    final result = applyWritingEdits(editor, [
      {
        'start': 8,
        'end': 17,
        'expectedText': 'an letter',
        'replacement': 'a letter',
      },
    ]);
    final ops = documentToDelta(result).toJson();
    expect(
      ops.any(
        (op) => op['insert'] == 'letter' && op['attributes']?['italic'] == true,
      ),
      isTrue,
    );
  });
  test(
    'small corrections preserve rich formatting, emoji and unrelated blocks',
    () {
      final editor = editorFor(
        '😀 We recieved the letter.\nKeep this paragraph.',
      );
      addTearDown(editor.dispose);
      editor.formatText(6, 8, q.Attribute.bold);
      final before = editor.document.toDelta().toJson();
      final revised = applyWritingEdits(editor, [
        {
          'start': 6,
          'end': 14,
          'expectedText': 'recieved',
          'replacement': 'received',
        },
      ]);
      final copy = q.Document.fromDelta(documentToDelta(revised));
      expect(
        copy.toPlainText(),
        '😀 We received the letter.\nKeep this paragraph.\n',
      );
      expect(
        copy.toDelta().toJson().any(
          (op) =>
              op['insert'] == 'received' && op['attributes']?['bold'] == true,
        ),
        isTrue,
      );
      expect(editor.document.toDelta().toJson(), before);
      expect(
        () => applyWritingEdits(editor, [
          {
            'start': 6,
            'end': 14,
            'expectedText': 'changed!',
            'replacement': 'x',
          },
        ]),
        throwsStateError,
      );
      expect(
        () => applyWritingEdits(editor, [
          {
            'start': 1,
            'end': 2,
            'expectedText': editor.document.toPlainText().substring(1, 2),
            'replacement': 'x',
          },
        ]),
        throwsStateError,
      );
      expect(
        () => applyWritingEdits(editor, [
          {
            'start': 6,
            'end': 14,
            'expectedText': 'recieved',
            'replacement': 'x',
          },
          {
            'start': 6,
            'end': 14,
            'expectedText': 'recieved',
            'replacement': 'y',
          },
        ]),
        throwsStateError,
      );
    },
  );
  test(
    'AI corrections are unique, scoped and rejected when revision changes',
    () {
      final editor = editorFor('One recieved letter.\nTwo recieved letters.');
      addTearDown(editor.dispose);
      final review = WritingReviewSnapshot.capture(
        editor,
        4,
        'workspace',
        start: 0,
        end: 20,
      );
      expect(review.blocks.length, 1);
      final request = [
        {
          'blockId': 'p0',
          'expectedText': 'recieved',
          'replacement': 'received',
        },
      ];
      final edits = review.resolve(editor, 4, 'workspace', request);
      final result = q.Document.fromDelta(
        documentToDelta(applyWritingEdits(editor, edits)),
      ).toPlainText();
      expect(result, 'One received letter.\nTwo recieved letters.\n');
      expect(
        () => review.resolve(editor, 5, 'workspace', request),
        throwsStateError,
      );
      expect(
        () => review.resolve(editor, 4, 'other', request),
        throwsStateError,
      );
      expect(
        () => review.resolve(editor, 4, 'workspace', [
          {
            'blockId': 'p21',
            'expectedText': 'recieved',
            'replacement': 'received',
          },
        ]),
        throwsStateError,
      );
      editor.replaceText(0, 0, 'New ', null);
      expect(
        () => review.resolve(editor, 4, 'workspace', request),
        throwsStateError,
      );
    },
  );
  test('code and links are excluded without shifting UTF16 positions', () {
    final editor = editorFor(
      'recieved https://example.com 😀\nrecieved code\nrecieved',
    );
    addTearDown(editor.dispose);
    final text = editor.document.toPlainText();
    final code = text.indexOf('recieved code');
    editor.formatText(code, 13, q.Attribute.codeBlock);
    final masked = ProofreadingController.checkableText(editor.document);
    expect(masked.length, text.length);
    expect(masked.contains('https:'), isFalse);
    expect(masked.contains('recieved code'), isFalse);
    expect(masked.lastIndexOf('recieved'), text.lastIndexOf('recieved'));
    expect(
      () => applyWritingEdits(editor, [
        {
          'start': code,
          'end': code + 8,
          'expectedText': 'recieved',
          'replacement': 'received',
        },
      ]),
      throwsStateError,
    );
  });
  test(
    'paragraph cache and personal dictionary persist, forget restores issue',
    () async {
      final dir = await Directory.systemTemp.createTemp('folio-proof-test-');
      final editor = editorFor('We recieved it.\nWe recieved it.');
      var calls = 0;
      final controller = ProofreadingController(
        editor,
        settingsPath: '${dir.path}/proofreading.json',
        check: (r) async {
          calls++;
          return resultFor(r['text']);
        },
      );
      await Future<void>.delayed(const Duration(milliseconds: 20));
      await controller.checkNow();
      expect(controller.issues.length, 2);
      expect(calls, 1);
      await controller.learn('recieved');
      await controller.checkNow();
      expect(controller.issues, isEmpty);
      expect(calls, 1);
      expect(
        jsonDecode(
          await File('${dir.path}/proofreading.json').readAsString(),
        )['words'],
        contains('recieved'),
      );
      await controller.forget('recieved');
      await controller.checkNow();
      expect(controller.issues.length, 2);
      controller.ignore(controller.issues.first);
      expect(controller.issues.length, 1);
      controller.dispose();
      editor.dispose();
      await dir.delete(recursive: true);
    },
  );
  testWidgets(
    'superseded check does not leave a debounce timer after disposal',
    (tester) async {
      final editor = editorFor('We recieved it.');
      final pending = Completer<Map<String, dynamic>>();
      final controller = ProofreadingController(
        editor,
        check: (_) => pending.future,
      );
      final first = controller.checkNow();
      editor.replaceText(3, 8, 'received', null);
      pending.complete(resultFor('We recieved it.'));
      await first;
      controller.dispose();
      editor.dispose();
      // The widget test's pending-timer invariant also catches a timer whose
      // reference was overwritten by the retry timer before dispose.
    },
  );
  test('results arriving after typing are discarded', () async {
    final editor = editorFor('We recieved it.');
    final pending = Completer<Map<String, dynamic>>();
    var calls = 0;
    final controller = ProofreadingController(
      editor,
      check: (r) {
        calls++;
        return calls == 1 ? pending.future : Future.value(resultFor(r['text']));
      },
    );
    final first = controller.checkNow();
    editor.replaceText(3, 8, 'received', null);
    pending.complete(resultFor('We recieved it.'));
    await first;
    expect(controller.issues, isEmpty);
    expect(controller.isCurrent(), isFalse);
    await controller.checkNow();
    expect(controller.isCurrent(), isTrue);
    expect(controller.issues, isEmpty);
    controller.dispose();
    editor.dispose();
  });
}
