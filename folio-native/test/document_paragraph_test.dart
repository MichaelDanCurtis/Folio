import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_lists.dart';
import 'package:folio_native/src/document_list_rules.dart';
import 'package:folio_native/src/document_paragraph_format.dart';
import 'package:folio_native/src/document_styles.dart';
import 'package:folio_native/src/document_tabs.dart';
import 'package:folio_native/src/document_table_view.dart';

Map<String, dynamic> format(Map<String, dynamic> direct) => {
  'inherited': {'before': 0, 'after': 0, 'line': 240, 'lineRule': 'auto'},
  'direct': direct,
  'styleId': 'Body',
};
Map<String, dynamic> metadata(Map<String, dynamic> direct) => {
  'officeId': 'p0',
  'paragraphFormat': format(direct),
  'tabs': {
    'inherited': [],
    'direct': [
      {'position': 1440, 'alignment': 'left', 'leader': 'none'},
    ],
  },
};
Delta sample(Map<String, dynamic> direct) => Delta()
  ..insert('Alpha\tBeta', {'bold': true})
  ..insert('\n', {'token': jsonEncode(metadata(direct))})
  ..insert('Second')
  ..insert('\n', {
    'token': jsonEncode({
      'officeId': 'p1',
      'paragraphFormat': format({'start': 360, 'end': 180}),
    }),
  });

void main() {
  testWidgets(
    'aligned and listed blocks collapse spacing without a second outer gap',
    (tester) async {
      for (final block in [
        <String, dynamic>{},
        {'align': 'center'},
        {'list': 'ordered'},
        {'header': 2},
      ]) {
        final delta = Delta()
          ..insert('First')
          ..insert('\n', {
            ...block,
            'token': jsonEncode({
              'paragraphFormat': format({'before': 120, 'after': 300}),
            }),
          })
          ..insert('Second')
          ..insert('\n', {
            ...block,
            'token': jsonEncode({
              'paragraphFormat': format({'before': 180, 'after': 0}),
            }),
          });
        final controller = q.QuillController(
          document: q.Document.fromDelta(delta),
          selection: const TextSelection.collapsed(offset: 0),
        );
        await tester.pumpWidget(
          MaterialApp(
            localizationsDelegates: const [
              q.FlutterQuillLocalizations.delegate,
            ],
            builder: (context, child) => q.QuillParagraphLayout(
              builder: layoutDocumentTabs,
              paddingBuilder: paragraphPadding,
              styleBuilder: paragraphTextStyle,
              strutBuilder: paragraphStrut,
              blockSpacingBuilder: paragraphBlockSpacing,
              child: child!,
            ),
            home: Scaffold(
              body: q.QuillEditor.basic(
                controller: controller,
                config: q.QuillEditorConfig(
                  scrollable: false,
                  customStyles: documentEditorStyles(),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final rendered = tester
            .renderObjectList<RenderParagraph>(
              find.byWidgetPredicate((w) => w is RichText),
            )
            .where((r) => ['First', 'Second'].contains(r.text.toPlainText()))
            .toList();
        expect(rendered.length, 2);
        expect(
          rendered[1].localToGlobal(Offset.zero).dy -
              rendered[0].localToGlobal(Offset.zero).dy -
              rendered[0].size.height,
          closeTo(20, .1),
          reason: '$block',
        );
        await tester.pumpWidget(const SizedBox.shrink());
        controller.dispose();
      }
    },
  );
  testWidgets(
    'static table paragraphs render their indents, collapsed spacing and exact height',
    (tester) async {
      final node = {
        'type': 'table',
        'content': [
          {
            'type': 'tableRow',
            'content': [
              {
                'type': 'tableCell',
                'content': [
                  {
                    'type': 'paragraph',
                    'attrs': {
                      'paragraphFormat': format({
                        'left': 360,
                        'after': 240,
                        'line': 480,
                        'lineRule': 'exact',
                      }),
                    },
                    'content': [
                      {'type': 'text', 'text': 'First'},
                    ],
                  },
                  {
                    'type': 'paragraph',
                    'attrs': {
                      'paragraphFormat': format({'left': 180, 'before': 120}),
                    },
                    'content': [
                      {'type': 'text', 'text': 'Second'},
                    ],
                  },
                ],
              },
            ],
          },
        ],
      };
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(width: 400, child: DocumentTableView(node: node)),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final first = tester.getRect(find.text('First', findRichText: true));
      final second = tester.getRect(find.text('Second', findRichText: true));
      expect(first.left - second.left, closeTo(12, .1));
      expect(first.height, closeTo(32, .1));
      expect(second.top - first.bottom, closeTo(16, .1));
      expect(tester.takeException(), isNull);
    },
  );
  test(
    'native paragraphs retain their current spacing when only the indent changes',
    () {
      final source = Delta()..insert('Native paragraph\n');
      final next = source.compose(
        changeParagraphFormat(
          source,
          const TextSelection.collapsed(offset: 0),
          {'left': 720},
        ),
      );
      expect(
        listMetadata(listLines(next).single.attrs['token'])['paragraphFormat'],
        {
          'inherited': {},
          'direct': {
            'left': 720,
            'before': 0,
            'after': 180,
            'line': 336,
            'lineRule': 'auto',
          },
        },
      );
    },
  );
  test(
    'formatting selected paragraphs retains rich text and metadata without changing offsets',
    () {
      final original = sample({'left': 360});
      final change = changeParagraphFormat(
        original,
        const TextSelection(baseOffset: 0, extentOffset: 11),
        {'left': 720, 'before': 120},
      );
      final next = original.compose(change);
      expect(next.slice(0, 10), original.slice(0, 10));
      final lines = listLines(next);
      final token = listMetadata(lines[0].attrs['token']);
      expect(token['officeId'], 'p0');
      expect(token['tabs'], metadata({})['tabs']);
      expect(paragraphProperties(token['paragraphFormat'])['left'], 720);
      expect(lines[1].attrs, listLines(original)[1].attrs);
      expect(
        deltaToDocument(documentToDelta(deltaToDocument(next))),
        deltaToDocument(next),
      );
      final both = original.compose(
        changeParagraphFormat(
          original,
          const TextSelection(baseOffset: 0, extentOffset: 17),
          {'left': 900, 'right': 300},
        ),
      );
      final second = paragraphProperties(
        listMetadata(listLines(both)[1].attrs['token'])['paragraphFormat'],
      );
      expect(second['start'], 900);
      expect(second['end'], 300);
      expect(
        changeParagraphFormat(
          both,
          const TextSelection(baseOffset: 0, extentOffset: 17),
          {'left': 900, 'right': 300},
        ).isEmpty,
        isTrue,
      );
      final cleared = both.compose(
        changeParagraphFormat(both, const TextSelection.collapsed(offset: 11), {
          'left': null,
        }),
      );
      final direct =
          listMetadata(
                listLines(cleared)[1].attrs['token'],
              )['paragraphFormat']['direct']
              as Map;
      expect(direct.containsKey('start'), isFalse);
      expect(direct.containsKey('left'), isFalse);
    },
  );
  test(
    'Enter and multiline insertion copy formatting and keep a unique source identity',
    () {
      for (final text in ['\n', 'one\ntwo\nthree']) {
        final document = q.Document.fromDelta(
          sample({'left': 540, 'after': 120}),
        );
        configureDocumentLists(document);
        document.insert(5, text);
        final lines = listLines(document.toDelta());
        final copied = lines
            .take(text.split('\n').length)
            .map((l) => listMetadata(l.attrs['token']))
            .toList();
        expect(copied.first['officeId'], 'p0');
        expect(copied.skip(1).every((m) => !m.containsKey('officeId')), isTrue);
        for (final m in copied) {
          expect(
            m['paragraphFormat'],
            metadata({'left': 540, 'after': 120})['paragraphFormat'],
          );
          expect(m['tabs'], metadata({})['tabs']);
        }
        document.close();
      }
    },
  );
  test(
    'default and custom tab stops stay at page-text-margin positions after an indent',
    () {
      for (final custom in [false, true]) {
        const source = TextSpan(text: 'A\tB', style: TextStyle(fontSize: 16));
        final span = layoutTabStops(
          source,
          maxWidth: 300,
          origin: 24,
          interval: 48,
          stops: custom ? [const DocumentTabStop(1440)] : [],
        );
        final painter = TextPainter(
          text: span,
          textDirection: TextDirection.ltr,
        )..layout(maxWidth: 300);
        expect(span.toPlainText(), 'A\tB');
        expect(
          painter
                  .getOffsetForCaret(const TextPosition(offset: 2), Rect.zero)
                  .dx +
              24,
          closeTo(custom ? 96 : 48, .05),
        );
        painter.dispose();
      }
    },
  );
  testWidgets(
    'indent, spacing collapse, wrapping, caret and hit testing use the same paragraph geometry',
    (tester) async {
      final document = q.Document.fromDelta(
        Delta()
          ..insert('A\tB')
          ..insert('\n', {
            'token': jsonEncode({
              'paragraphFormat': format({
                'left': 360,
                'right': 180,
                'before': 120,
                'after': 300,
              }),
            }),
          })
          ..insert(
            'Second paragraph wraps across several visible rows with a right indent.',
          )
          ..insert('\n', {
            'token': jsonEncode({
              'paragraphFormat': format({
                'left': 540,
                'right': 720,
                'before': 180,
                'after': 0,
              }),
            }),
          }),
      );
      final controller = q.QuillController(
        document: document,
        selection: const TextSelection.collapsed(offset: 0),
      );
      final focus = FocusNode();
      final scroll = ScrollController();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        controller.dispose();
        focus.dispose();
        scroll.dispose();
      });
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          builder: (context, child) => q.QuillParagraphLayout(
            builder: layoutDocumentTabs,
            paddingBuilder: paragraphPadding,
            styleBuilder: paragraphTextStyle,
            strutBuilder: paragraphStrut,
            blockSpacingBuilder: paragraphBlockSpacing,
            child: child!,
          ),
          home: Scaffold(
            body: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 320,
                child: q.QuillEditor.basic(
                  controller: controller,
                  focusNode: focus,
                  scrollController: scroll,
                  config: q.QuillEditorConfig(
                    scrollable: false,
                    customStyles: documentEditorStyles(),
                  ),
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
      Rect caret(int i) =>
          raw.renderEditor.getLocalRectForCaret(TextPosition(offset: i));
      expect(caret(0).left, closeTo(24, 1));
      expect(caret(2).left, closeTo(48, 1));
      expect(caret(4).left, closeTo(36, 1));
      final paras = tester
          .renderObjectList<RenderParagraph>(
            find.byWidgetPredicate((w) => w is RichText),
          )
          .where(
            (r) =>
                r.text.toPlainText().startsWith('A\t') ||
                r.text.toPlainText().startsWith('Second'),
          )
          .toList();
      expect(paras.length, 2);
      final first = paras[0].localToGlobal(Offset.zero);
      final second = paras[1].localToGlobal(Offset.zero);
      expect(first.dy, closeTo(8, .1));
      expect(second.dy - first.dy - paras[0].size.height, closeTo(20, .1));
      expect(paras[1].constraints.maxWidth, closeTo(320 - 36 - 48, .1));
      final target = caret(2);
      await tester.tapAt(
        raw.renderEditor.localToGlobal(Offset(target.left, target.center.dy)),
      );
      await tester.pumpAndSettle();
      expect(controller.selection.baseOffset, 2);
      expect(document.toPlainText().startsWith('A\tB\n'), isTrue);
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
  test('auto exact and minimum line height respond to mixed font sizes', () {
    double height(String rule, int line, {double large = 16}) {
      final p = {'line': line, 'lineRule': rule};
      final style = paragraphStyle(p, const TextStyle(fontSize: 16));
      final painter = TextPainter(
        text: TextSpan(
          style: style,
          children: [
            TextSpan(
              text: 'Wide text\nnext row',
              style: TextStyle(fontSize: large),
            ),
          ],
        ),
        strutStyle: paragraphStrutStyle(p, StrutStyle.fromTextStyle(style)),
        textDirection: TextDirection.ltr,
      )..layout(maxWidth: 1000);
      final result = painter.height;
      painter.dispose();
      return result;
    }

    expect(height('auto', 480), closeTo(height('auto', 240) * 2, .1));
    expect(height('exact', 480, large: 48), closeTo(64, .1));
    expect(height('atLeast', 480), closeTo(64, .1));
    expect(height('atLeast', 480, large: 48), greaterThan(64));
  });
}
