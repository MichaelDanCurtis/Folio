import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_links.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  test('only web, mail and in-document links are followed', () {
    expect(linkKind('https://example.org/a'), LinkKind.web);
    expect(linkKind('HTTP://example.org'), LinkKind.web);
    expect(linkKind('mailto:a@example.org'), LinkKind.mail);
    expect(linkKind('#_Intro'), LinkKind.internal);
    for (final blocked in [
      'file:///etc/passwd',
      'javascript:alert(1)',
      'ftp://example.org',
      'https://',
      'smb://server/share',
    ]) {
      expect(linkKind(blocked), LinkKind.blocked, reason: blocked);
    }
  });

  test('addresses are completed as Word does', () {
    expect(normalizeLinkAddress(' example.org/x '), 'https://example.org/x');
    expect(normalizeLinkAddress('a.b@example.org'), 'mailto:a.b@example.org');
    expect(normalizeLinkAddress('http://example.org'), 'http://example.org');
    expect(normalizeLinkAddress('javascript:alert(1)'), isNull);
    expect(normalizeLinkAddress('two words'), isNull);
    expect(normalizeLinkAddress(''), isNull);
  });

  test('link ranges span runs with the same address', () {
    final doc = Delta()
      ..insert('see ')
      ..insert('the ', {'link': 'https://a.org'})
      ..insert('docs', {'link': 'https://a.org', 'bold': true})
      ..insert(' now\n');
    expect(linkRangeAt(doc, 6), const TextRange(start: 4, end: 12));
    expect(linkRangeAt(doc, 12), const TextRange(start: 4, end: 12));
    expect(linkRangeAt(doc, 2), isNull);
    expect(linkAt(doc, 9), 'https://a.org');
  });

  test('setting a link keeps unchanged runs and replaces edited text', () {
    final doc = Delta()
      ..insert('plain ', {'italic': true})
      ..insert('text\n');
    expect(
      doc
          .compose(setLinkDelta(doc, 0, 5, 'plain', 'https://a.org'))
          .toJson()
          .first,
      {
        'insert': 'plain',
        'attributes': {'italic': true, 'link': 'https://a.org'},
      },
    );
    expect(
      doc.compose(setLinkDelta(doc, 6, 6, 'Folio', 'https://a.org')).toJson(),
      [
        {
          'insert': 'plain ',
          'attributes': {'italic': true},
        },
        {
          'insert': 'Folio',
          'attributes': {'link': 'https://a.org'},
        },
        {'insert': 'text\n'},
      ],
    );
  });

  test('heading bookmarks are hidden, valid and unique', () {
    expect(
      headingBookmarkName('Quarterly results: 2026', []),
      '_Quarterly_results_2026',
    );
    expect(headingBookmarkName('Intro', ['_intro']), '_Intro2');
    expect(headingBookmarkName('!!!', []), '_Heading');
    expect(headingBookmarkName('x' * 60, []).length, 36);
  });

  testWidgets(
    'Ctrl+K inserts, edits and removes; Ctrl+click follows',
    (t) async {
      final opened = <Uri>[];
      final previous = openExternalLink;
      openExternalLink = (uri) async => opened.add(uri);
      addTearDown(() => openExternalLink = previous);
      final controller = q.QuillController(
        document: q.Document.fromDelta(Delta()..insert('Read the guide\n')),
        selection: const TextSelection(baseOffset: 9, extentOffset: 14),
      );
      addTearDown(controller.dispose);
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: DocumentView(
              controller: controller,
              undo: () {},
              redo: () {},
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      t
          .widget<q.QuillEditor>(find.byType(q.QuillEditor))
          .focusNode
          .requestFocus();
      controller.updateSelection(
        const TextSelection(baseOffset: 9, extentOffset: 14),
        q.ChangeSource.local,
      );
      await t.pump();
      Future<void> shortcut() async {
        await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
        await t.sendKeyEvent(LogicalKeyboardKey.keyK);
        await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
        await t.pumpAndSettle();
      }

      await shortcut();
      expect(find.text('Insert hyperlink'), findsOneWidget);
      await t.enterText(find.byKey(const Key('link-address')), 'javascript:x');
      await t.tap(find.byKey(const Key('link-apply')));
      await t.pumpAndSettle();
      expect(find.textContaining('Enter a web address'), findsOneWidget);
      await t.enterText(find.byKey(const Key('link-address')), 'example.org');
      await t.tap(find.byKey(const Key('link-apply')));
      await t.pumpAndSettle();
      expect(linkAt(controller.document.toDelta(), 10), 'https://example.org');
      expect(controller.document.toPlainText(), 'Read the guide\n');

      controller.updateSelection(
        const TextSelection.collapsed(offset: 11),
        q.ChangeSource.local,
      );
      await shortcut();
      expect(find.text('Edit hyperlink'), findsOneWidget);
      await t.enterText(find.byKey(const Key('link-text')), 'manual');
      await t.tap(find.byKey(const Key('link-apply')));
      await t.pumpAndSettle();
      expect(controller.document.toPlainText(), 'Read the manual\n');
      expect(
        linkRangeAt(controller.document.toDelta(), 10),
        const TextRange(start: 9, end: 15),
      );

      // Ctrl+click on the link follows it.
      final editor = t.getRect(find.byType(q.QuillEditor));
      final caret = t
          .state<q.QuillRawEditorState>(find.byType(q.QuillRawEditor))
          .renderEditor
          .getLocalRectForCaret(const TextPosition(offset: 12));
      final box = t.renderObject<RenderBox>(find.byType(q.QuillRawEditor));
      final point = box.localToGlobal(caret.center);
      expect(editor.contains(point), true);
      await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await t.pump();
      await t.tapAt(point);
      await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await t.pumpAndSettle();
      expect(opened, [Uri.parse('https://example.org')]);

      final view = t.state<DocumentViewState>(find.byType(DocumentView));
      await view.followLink('file:///etc/passwd');
      await t.pump();
      expect(find.textContaining('only follows web'), findsOneWidget);
      expect(opened, hasLength(1));

      controller.updateSelection(
        const TextSelection.collapsed(offset: 11),
        q.ChangeSource.local,
      );
      await shortcut();
      await t.tap(find.byKey(const Key('link-remove')));
      await t.pumpAndSettle();
      expect(linkRangeAt(controller.document.toDelta(), 11), isNull);
      expect(controller.document.toPlainText(), 'Read the manual\n');
    },
    variant: TargetPlatformVariant.only(TargetPlatform.linux),
  );
}
