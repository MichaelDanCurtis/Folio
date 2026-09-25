// Word Count (W12.09): pages, words, characters, paragraphs and lines for
// the document or the selection, optionally with text boxes and notes.
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'document_bookmarks.dart';
import 'document_page_layout.dart';

class WordCountIntent extends Intent {
  const WordCountIntent();
}

const wordCountShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.keyG, control: true, shift: true):
      WordCountIntent(),
};

class TextStatistics {
  final int words, characters, charactersWithSpaces, paragraphs;
  const TextStatistics(
    this.words,
    this.characters,
    this.charactersWithSpaces,
    this.paragraphs,
  );

  /// Counts [paragraphs] as Word does: characters exclude paragraph marks and
  /// line breaks, and empty paragraphs are not counted.
  factory TextStatistics.of(Iterable<String> paragraphs) {
    var words = 0, characters = 0, withSpaces = 0, count = 0;
    for (final raw in paragraphs) {
      // Objects Folio keeps as placeholders are not text.
      final p = raw.replaceAll('￼', '');
      words += RegExp(r'\S+').allMatches(p).length;
      final visible = p.replaceAll('\n', '');
      withSpaces += visible.characters.length;
      characters += visible.characters.where((c) => c.trim().isNotEmpty).length;
      if (p.trim().isNotEmpty) count++;
    }
    return TextStatistics(words, characters, withSpaces, count);
  }
}

/// The body's paragraphs, including those in tables.
List<String> documentParagraphs(Map document) => [
  for (final p in DocumentBookmarkIndex(document).paragraphs) p.text,
];

/// Footnote, endnote and text box paragraphs from `documentNotesText`.
List<String> notesParagraphs(Map notes) => [
  for (final key in ['textBoxes', 'footnotes', 'endnotes'])
    for (final note in notes[key] as List? ?? [])
      ...(note as String).split('\n'),
];

class DocumentWordCountDialog extends StatefulWidget {
  final List<String> document;

  /// The selected text, or null when the whole document is counted.
  final String? selection;
  final List<String> notes;

  /// The `documentPagePreview` snapshot, for pages and lines.
  final Map? layout;
  const DocumentWordCountDialog({
    required this.document,
    required this.notes,
    this.selection,
    this.layout,
    super.key,
  });
  @override
  State<DocumentWordCountDialog> createState() =>
      _DocumentWordCountDialogState();
}

class _DocumentWordCountDialogState extends State<DocumentWordCountDialog> {
  bool includeNotes = true;
  int? pages, lines;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (widget.layout == null || pages != null) return;
    final locale = Localizations.maybeLocaleOf(context);
    Future<void>(() {
      if (!mounted) return;
      try {
        final layout = DocumentPageLayout.build(widget.layout!, locale: locale);
        final counts = (layout.pages.length, layout.lineCount);
        layout.dispose();
        if (mounted) {
          setState(() {
            pages = counts.$1;
            lines = counts.$2;
          });
        }
      } catch (_) {
        // Pages and lines stay unavailable; the text counts are still right.
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final selected = widget.selection;
    final stats = TextStatistics.of([
      if (selected != null) ...selected.split('\n') else ...widget.document,
      if (includeNotes && selected == null) ...widget.notes,
    ]);
    String number(int? n) => n == null
        ? '…'
        : n.toString().replaceAllMapped(RegExp(r'\B(?=(\d{3})+$)'), (_) => ',');
    final rows = [
      if (selected == null) ('Pages', pages, 'word-count-pages'),
      ('Words', stats.words, 'word-count-words'),
      ('Characters (no spaces)', stats.characters, 'word-count-characters'),
      (
        'Characters (with spaces)',
        stats.charactersWithSpaces,
        'word-count-characters-spaces',
      ),
      ('Paragraphs', stats.paragraphs, 'word-count-paragraphs'),
      if (selected == null) ('Lines', lines, 'word-count-lines'),
    ];
    return AlertDialog(
      title: const Text('Word Count'),
      content: SizedBox(
        width: 320,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              selected == null ? 'Document' : 'Selection',
              style: Theme.of(context).textTheme.labelLarge,
            ),
            const SizedBox(height: 8),
            for (final (label, value, key) in rows)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 3),
                child: Row(
                  children: [
                    Expanded(child: Text(label)),
                    Text(number(value), key: Key(key)),
                  ],
                ),
              ),
            const SizedBox(height: 8),
            CheckboxListTile(
              key: const Key('word-count-include-notes'),
              contentPadding: EdgeInsets.zero,
              dense: true,
              value: includeNotes,
              onChanged: selected == null
                  ? (v) => setState(() => includeNotes = v ?? false)
                  : null,
              title: const Text('Include text boxes, footnotes and endnotes'),
            ),
          ],
        ),
      ),
      actions: [
        FilledButton(
          key: const Key('word-count-close'),
          onPressed: () => Navigator.pop(context),
          child: const Text('Close'),
        ),
      ],
    );
  }
}
