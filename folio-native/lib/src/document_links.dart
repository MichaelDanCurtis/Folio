// Hyperlinks in documents (W01.14): insert and edit with Ctrl/Cmd+K, remove,
// and follow with Ctrl/Cmd+click. Internal links are `#bookmark`, as in DOCX.
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';
import 'document_bookmarks.dart';
import 'draft_text_field.dart';

class EditLinkIntent extends Intent {
  const EditLinkIntent();
}

const linkShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.keyK, meta: true): EditLinkIntent(),
  SingleActivator(LogicalKeyboardKey.keyK, control: true): EditLinkIntent(),
};

/// Opens a web or mail link in the platform's handler; replaced in tests.
Future<void> Function(Uri) openExternalLink = (uri) =>
    launchUrl(uri, mode: LaunchMode.externalApplication);

enum LinkKind { web, mail, internal, blocked }

/// Only web, mail and in-document links are followed.
LinkKind linkKind(String href) {
  if (href.startsWith('#')) return LinkKind.internal;
  final uri = Uri.tryParse(href);
  return switch (uri?.scheme.toLowerCase()) {
    'http' || 'https' when uri!.host.isNotEmpty => LinkKind.web,
    'mailto' when uri!.path.contains('@') => LinkKind.mail,
    _ => LinkKind.blocked,
  };
}

/// What was typed as an address, as Word completes it: `name@host` becomes
/// `mailto:`, and a bare host gets `https://`. Null when it can't be used.
String? normalizeLinkAddress(String input) {
  final value = input.trim();
  if (value.isEmpty || value.contains(RegExp(r'\s'))) return null;
  if (value.startsWith('#')) return value.length > 1 ? value : null;
  if (RegExp(r'^[^@/:]+@[^@/]+\.[^@/]+$').hasMatch(value)) {
    return 'mailto:$value';
  }
  final full = RegExp(r'^[a-zA-Z][a-zA-Z0-9+.-]*:').hasMatch(value)
      ? value
      : 'https://$value';
  return linkKind(full) == LinkKind.blocked ? null : full;
}

/// The run of text carrying the same link as [offset], or null.
TextRange? linkRangeAt(Delta document, int offset) {
  final ops = document.toList();
  var at = 0;
  for (var i = 0; i < ops.length; i++) {
    final end = at + ops[i].length!;
    final href = ops[i].attributes?['link'];
    if (href != null && offset >= at && (offset < end || offset == end)) {
      var start = at, stop = end;
      for (var j = i - 1; j >= 0 && ops[j].attributes?['link'] == href; j--) {
        start -= ops[j].length!;
      }
      for (
        var j = i + 1;
        j < ops.length && ops[j].attributes?['link'] == href;
        j++
      ) {
        stop += ops[j].length!;
      }
      return TextRange(start: start, end: stop);
    }
    at = end;
  }
  return null;
}

String? linkAt(Delta document, int offset) {
  final range = linkRangeAt(document, offset);
  if (range == null) return null;
  return document.slice(range.start, range.end).first.attributes?['link'];
}

/// Sets [start]..[end] to [text] linked to [href]. Text that is unchanged
/// keeps its runs; replaced text takes the first run's formatting.
Delta setLinkDelta(
  Delta document,
  int start,
  int end,
  String text,
  String href,
) {
  final slice = document.slice(start, end).toList();
  final current = slice.map((op) => op.data is String ? op.data : '').join();
  final change = Delta()..retain(start);
  if (end > start &&
      current == text &&
      slice.every((op) => op.data is String) &&
      !current.contains('\n')) {
    return change
      ..retain(end - start, {'link': href})
      ..trim();
  }
  final attrs = {...?slice.firstOrNull?.attributes, 'link': href};
  if (end > start) change.delete(end - start);
  return change..insert(text, attrs);
}

Delta removeLinkDelta(TextRange range) => Delta()
  ..retain(range.start)
  ..retain(range.end - range.start, {'link': null});

/// A place in this document a link can go to.
class LinkPlace {
  final String label;

  /// An existing bookmark's name, or null for a heading that gets a hidden
  /// bookmark when it is first linked to.
  final String? bookmark;
  final int start, end;
  final bool heading;
  const LinkPlace(
    this.label,
    this.bookmark,
    this.start,
    this.end, {
    this.heading = false,
  });
}

List<LinkPlace> linkPlaces(Map document) {
  final bookmarks = documentBookmarks(document);
  final index = DocumentBookmarkIndex(document);
  final content = document['content'] as List? ?? [];
  return [
    for (final p in index.paragraphs)
      if (p.path.length == 1 &&
          content[p.path.first]['type'] == 'heading' &&
          p.text.trim().isNotEmpty)
        LinkPlace(
          p.text.trim(),
          bookmarks
              .where((b) => b['start'] == p.start && b['end'] == p.end)
              .map((b) => b['name'] as String)
              .firstOrNull,
          p.start,
          p.end,
          heading: true,
        ),
    for (final b in bookmarks)
      if (!(b['name'] as String).startsWith('_'))
        LinkPlace(
          b['name'] as String,
          b['name'] as String,
          b['start'] as int,
          b['end'] as int,
        ),
  ];
}

/// Word's hidden bookmark for a heading: `_` and the heading's words.
String headingBookmarkName(String heading, Iterable<String> taken) {
  final used = {for (final t in taken) t.toLowerCase()};
  var base =
      '_${heading.trim().replaceAll(RegExp(r'[^\p{L}\p{N}]+', unicode: true), '_')}';
  base = base.replaceAll(RegExp(r'_+$'), '');
  if (base.length < 2) base = '_Heading';
  base = String.fromCharCodes(base.runes.take(36));
  var name = base;
  for (var n = 2; used.contains(name.toLowerCase()); n++) {
    name = '$base$n';
  }
  return name;
}

class LinkResult {
  final String text, href;
  final LinkPlace? place;
  final bool remove;
  const LinkResult(this.text, this.href, {this.place}) : remove = false;
  const LinkResult.remove() : text = '', href = '', place = null, remove = true;
}

class DocumentLinkDialog extends StatefulWidget {
  final String text, address;
  final bool existing, textEditable;
  final List<LinkPlace> places;
  const DocumentLinkDialog({
    required this.text,
    required this.address,
    required this.places,
    this.existing = false,
    this.textEditable = true,
    super.key,
  });
  @override
  State<DocumentLinkDialog> createState() => _DocumentLinkDialogState();
}

class _DocumentLinkDialogState extends State<DocumentLinkDialog> {
  late final text = TextEditingController(text: widget.text);
  late final address = TextEditingController(
    text: widget.address.startsWith('#') ? '' : widget.address,
  );
  late bool inDocument = widget.address.startsWith('#');
  late LinkPlace? place = widget.places
      .where((p) => p.bookmark != null && '#${p.bookmark}' == widget.address)
      .firstOrNull;
  String? error;

  @override
  void dispose() {
    text.dispose();
    address.dispose();
    super.dispose();
  }

  void apply() {
    final String href;
    if (inDocument) {
      if (place == null) {
        setState(() => error = 'Choose a place in this document.');
        return;
      }
      href = place!.bookmark == null ? '' : '#${place!.bookmark}';
    } else {
      final normalized = normalizeLinkAddress(address.text);
      if (normalized == null) {
        setState(
          () => error =
              'Enter a web address (http or https) or an email address.',
        );
        return;
      }
      href = normalized;
    }
    final shown = text.text.isEmpty
        ? (inDocument ? place!.label : address.text.trim())
        : text.text;
    Navigator.pop(
      context,
      LinkResult(shown, href, place: inDocument ? place : null),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.existing ? 'Edit hyperlink' : 'Insert hyperlink'),
    content: SizedBox(
      width: 440,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          draftTextField(
            key: const Key('link-text'),
            controller: text,
            enabled: widget.textEditable,
            decoration: const InputDecoration(labelText: 'Text to display'),
          ),
          const SizedBox(height: 12),
          SegmentedButton<bool>(
            key: const Key('link-kind'),
            segments: const [
              ButtonSegment(value: false, label: Text('Web or email')),
              ButtonSegment(value: true, label: Text('Place in this document')),
            ],
            selected: {inDocument},
            onSelectionChanged: (v) => setState(() {
              inDocument = v.first;
              error = null;
            }),
          ),
          const SizedBox(height: 12),
          if (!inDocument)
            draftTextField(
              key: const Key('link-address'),
              controller: address,
              autofocus: true,
              decoration: InputDecoration(
                labelText: 'Address',
                hintText: 'https://example.com or name@example.com',
                errorText: error,
              ),
              onSubmitted: (_) => apply(),
            )
          else if (widget.places.isEmpty)
            const Text('Add a heading or a bookmark to link to it.')
          else ...[
            SizedBox(
              height: 220,
              child: ListView(
                children: [
                  for (final (i, p) in widget.places.indexed)
                    ListTile(
                      key: ValueKey('link-place-$i'),
                      dense: true,
                      leading: Icon(
                        p.heading ? Icons.title : Icons.bookmark_border,
                        size: 18,
                      ),
                      title: Text(
                        p.label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      selected: identical(place, p),
                      onTap: () => setState(() {
                        place = p;
                        error = null;
                      }),
                    ),
                ],
              ),
            ),
            if (error != null)
              Text(
                error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
          ],
        ],
      ),
    ),
    actions: [
      if (widget.existing)
        TextButton(
          key: const Key('link-remove'),
          onPressed: () => Navigator.pop(context, const LinkResult.remove()),
          child: const Text('Remove link'),
        ),
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('link-apply'),
        onPressed: apply,
        child: const Text('OK'),
      ),
    ],
  );
}
