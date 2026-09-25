import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_word_count.dart';

void main() {
  test('counts match Word', () {
    final stats = TextStatistics.of([
      'Two  words\tand a tab',
      '',
      'Line\nbreak ￼',
    ]);
    expect(stats.words, 7);
    expect(stats.characters, 'Twowordsandatab'.length + 'Linebreak'.length);
    expect(
      stats.charactersWithSpaces,
      'Two  words\tand a tab'.length + 'Linebreak '.length,
    );
    expect(stats.paragraphs, 2);
    expect(TextStatistics.of(['👍🏽 ok']).characters, 3);
  });

  test('notes are counted by paragraph', () {
    expect(
      notesParagraphs({
        'footnotes': ['One\nTwo'],
        'endnotes': ['End'],
        'textBoxes': ['Box'],
      }),
      ['Box', 'One', 'Two', 'End'],
    );
  });

  testWidgets('the checkbox adds text boxes and notes', (t) async {
    await t.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => const DocumentWordCountDialog(
                document: ['Body text here'],
                notes: ['A footnote'],
              ),
            ),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await t.tap(find.text('open'));
    await t.pumpAndSettle();
    String value(String key) => t.widget<Text>(find.byKey(Key(key))).data!;
    expect(value('word-count-words'), '5');
    expect(value('word-count-paragraphs'), '2');
    expect(value('word-count-pages'), '…');
    await t.tap(find.byKey(const Key('word-count-include-notes')));
    await t.pump();
    expect(value('word-count-words'), '3');
    expect(value('word-count-characters-spaces'), '14');
  });

  testWidgets('a selection is counted without pages or lines', (t) async {
    await t.pumpWidget(
      const MaterialApp(
        home: DocumentWordCountDialog(
          document: ['Ignored body'],
          notes: ['Ignored note'],
          selection: 'One two\nthree',
        ),
      ),
    );
    expect(find.text('Selection'), findsOneWidget);
    expect(find.byKey(const Key('word-count-pages')), findsNothing);
    expect(t.widget<Text>(find.byKey(const Key('word-count-words'))).data, '3');
  });
}
