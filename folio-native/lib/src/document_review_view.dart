import 'package:flutter/material.dart';
import 'document_revisions.dart';
import 'document_table_view.dart';
import 'document_tabs.dart';

/// A transient view of the retained Word source. View changes never enter the
/// edit queue; only explicit review actions mutate the document.
class DocumentReviewView extends StatefulWidget {
  final int revision;
  final Future<Map> Function(String mode) read;
  final Future<void> Function(Map<String, dynamic> request) review;
  final VoidCallback close;
  const DocumentReviewView({
    super.key,
    required this.revision,
    required this.read,
    required this.review,
    required this.close,
  });
  @override
  State<DocumentReviewView> createState() => _DocumentReviewViewState();
}

class _DocumentReviewViewState extends State<DocumentReviewView> {
  Map? data;
  String mode = 'markup';
  String? selected, author, error;
  bool busy = true;
  int serial = 0;
  final anchors = <String, GlobalKey>{};
  static const modes = {
    'markup': 'All markup',
    'current': 'Current',
    'original': 'Original',
  };
  List<Map> get records => (data?['revisions'] as List? ?? []).cast<Map>();
  List<Map> get rows =>
      records.where((r) => author == null || r['author'] == author).toList();
  Map? get current =>
      rows.where((r) => r['key'] == selected).firstOrNull ?? rows.firstOrNull;

  @override
  void initState() {
    super.initState();
    load(mode);
  }

  @override
  void didUpdateWidget(DocumentReviewView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.revision != widget.revision && !busy) load(mode);
  }

  Future<void> load(String requested, {int? nextIndex}) async {
    final generation = ++serial;
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final result = await widget.read(requested);
      if (!mounted || generation != serial) return;
      setState(() {
        data = result;
        mode = requested;
        anchors.clear();
        if (author != null && !records.any((r) => r['author'] == author)) {
          author = null;
        }
        if (nextIndex != null) {
          selected = rows.isEmpty
              ? null
              : rows[nextIndex.clamp(0, rows.length - 1)]['key'];
        }
        selected = current?['key'];
      });
      reveal();
    } catch (e) {
      if (mounted && generation == serial) setState(() => error = e.toString());
    } finally {
      if (mounted && generation == serial) setState(() => busy = false);
    }
  }

  void reveal() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final context = anchors[current?['officeId']]?.currentContext;
      if (context != null) {
        Scrollable.ensureVisible(
          context,
          alignment: .25,
          duration: const Duration(milliseconds: 180),
        );
      }
    });
  }

  void select(Map row) {
    setState(() => selected = row['key']);
    reveal();
  }

  Future<void> apply(bool accept) async {
    final row = current;
    if (busy ||
        row == null ||
        row[accept ? 'canAccept' : 'canReject'] != true) {
      return;
    }
    final index = rows.indexOf(row);
    setState(() {
      busy = true;
      error = null;
    });
    try {
      await widget.review({
        'workspaceId': data!['workspaceId'],
        'baseRevision': data!['baseRevision'],
        'action': accept ? 'accept' : 'reject',
        'keys': [row['key']],
      });
      if (!mounted) return;
      await load(mode, nextIndex: index);
    } catch (e) {
      if (mounted) {
        setState(() {
          busy = false;
          error = e.toString();
        });
      }
    }
  }

  Widget page() {
    final document = data!['view']['document'] as Map;
    final selectedBlock = current?['officeId'];
    return ColoredBox(
      color: const Color(0xffe8ebef),
      child: SingleChildScrollView(
        key: const Key('review-page-scroll'),
        padding: const EdgeInsets.all(20),
        child: Center(
          child: Container(
            constraints: const BoxConstraints(maxWidth: 816, minHeight: 1056),
            padding: const EdgeInsets.symmetric(horizontal: 36, vertical: 48),
            decoration: BoxDecoration(
              color: Colors.white,
              border: Border.all(color: const Color(0xffd1d5da)),
            ),
            child: DocumentTabSettings(
              intervalTwips: documentTabInterval(document),
              child: DocumentBlockContent(
                document,
                decorate: (block, content) {
                  final sources =
                      (block['attrs']?['reviewBlocks'] as List? ?? [])
                          .cast<String>();
                  final changes = rows
                      .where((r) => sources.contains(r['officeId']))
                      .toList();
                  final active =
                      selectedBlock != null && sources.contains(selectedBlock);
                  // Every source paragraph in a joined block reveals this block.
                  final key = GlobalKey();
                  for (final source in sources) {
                    anchors[source] = key;
                  }
                  return Semantics(
                    label: changes.isEmpty
                        ? null
                        : '${changes.length} tracked changes',
                    selected: active,
                    child: InkWell(
                      key: key,
                      onTap: busy || changes.isEmpty
                          ? null
                          : () => select(changes.first),
                      child: Container(
                        key: ValueKey('review-block-${sources.join('-')}'),
                        padding: const EdgeInsets.symmetric(
                          horizontal: 10,
                          vertical: 3,
                        ),
                        decoration: BoxDecoration(
                          color: active ? const Color(0xffedf3fc) : null,
                          border: Border(
                            left: BorderSide(
                              width: 3,
                              color: changes.isEmpty
                                  ? Colors.transparent
                                  : const Color(0xff185abd),
                            ),
                          ),
                        ),
                        child: content,
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget sidebar() {
    final row = current;
    final visible =
        row != null &&
        (data!['view']['document']['content'] as List).any(
          (n) => (n['attrs']?['reviewBlocks'] as List? ?? []).contains(
            row['officeId'],
          ),
        );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: DropdownButtonFormField<String>(
            key: ValueKey('review-author-$author'),
            initialValue: author ?? '\u0000',
            decoration: const InputDecoration(
              labelText: 'Reviewer',
              isDense: true,
              border: OutlineInputBorder(),
            ),
            items: [
              const DropdownMenuItem(
                value: '\u0000',
                child: Text('All authors'),
              ),
              ...records
                  .map((r) => r['author'] as String)
                  .toSet()
                  .toList()
                  .map(
                    (name) => DropdownMenuItem(
                      value: name,
                      child: Text(
                        name.isEmpty ? 'Unknown author' : name,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
            ],
            onChanged: busy
                ? null
                : (name) {
                    setState(() {
                      author = name == '\u0000' ? null : name;
                      selected = null;
                    });
                    reveal();
                  },
          ),
        ),
        Expanded(
          child: ListView(
            key: const Key('review-changes'),
            padding: const EdgeInsets.symmetric(horizontal: 10),
            children: [
              if (row == null)
                const Padding(
                  padding: EdgeInsets.all(12),
                  child: Text('No tracked changes in this document.'),
                ),
              if (row != null) ...[
                Text(
                  row['kind'],
                  style: const TextStyle(
                    fontSize: 17,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  '${row['author'].toString().isEmpty ? 'Unknown author' : row['author']} · ${row['story']}',
                ),
                if (row['date'].toString().isNotEmpty)
                  Text(row['date'], style: const TextStyle(fontSize: 11)),
                if (!visible)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Text(
                      row['officeId'] == null
                          ? 'This change is in ${row['story'].toString().toLowerCase()}, outside the main document page.'
                          : 'This change has no visible block in ${modes[mode]} view. Its preview remains below.',
                    ),
                  ),
                const SizedBox(height: 12),
                RevisionPreview(spans: row['preview']),
                if (row['move'] is Map) ...[
                  const Padding(
                    padding: EdgeInsets.only(top: 10),
                    child: Text(
                      'Accept or reject resolves the linked source and destination together.',
                    ),
                  ),
                  TextButton.icon(
                    key: const Key('review-page-linked-location'),
                    onPressed: busy
                        ? null
                        : () {
                            final linked = records
                                .where(
                                  (r) =>
                                      r['key'] == row['move']['counterpartKey'],
                                )
                                .firstOrNull;
                            if (linked != null) {
                              setState(() => author = null);
                              select(linked);
                            }
                          },
                    icon: const Icon(Icons.swap_horiz, size: 18),
                    label: const Text('Go to linked location'),
                  ),
                ],
                for (final format in row['formatChanges'] as List? ?? [])
                  Padding(
                    padding: const EdgeInsets.only(top: 8),
                    child: Text(
                      '${format['property']}\nPrevious: ${format['before']}\nCurrent: ${format['after']}',
                    ),
                  ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 6,
                  children: [
                    FilledButton.tonalIcon(
                      key: const Key('review-page-accept'),
                      onPressed: !busy && row['canAccept'] == true
                          ? () => apply(true)
                          : null,
                      icon: const Icon(Icons.check, size: 16),
                      label: const Text('Accept'),
                    ),
                    OutlinedButton.icon(
                      key: const Key('review-page-reject'),
                      onPressed: !busy && row['canReject'] == true
                          ? () => apply(false)
                          : null,
                      icon: const Icon(Icons.close, size: 16),
                      label: const Text('Reject'),
                    ),
                  ],
                ),
                if (row['canAccept'] != true)
                  Text('Accept unavailable: ${row['acceptReason']}'),
                if (row['canReject'] != true)
                  Text('Reject unavailable: ${row['rejectReason']}'),
                const Divider(height: 28),
              ],
              for (final r in rows)
                ListTile(
                  key: ValueKey('review-change-${r['key']}'),
                  dense: true,
                  selected: r['key'] == row?['key'],
                  selectedTileColor: const Color(0xffedf3fc),
                  title: Text(r['kind']),
                  subtitle: Text('${r['author']} · ${r['story']}'),
                  onTap: busy ? null : () => select(r),
                ),
            ],
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final index = rows.indexWhere((r) => r['key'] == current?['key']);
    return Column(
      children: [
        Material(
          color: const Color(0xfff5f7fb),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
            child: Wrap(
              spacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                DropdownButton<String>(
                  key: const Key('review-display-mode'),
                  value: mode,
                  items: modes.entries
                      .map(
                        (e) => DropdownMenuItem(
                          value: e.key,
                          child: Text(e.value),
                        ),
                      )
                      .toList(),
                  onChanged: busy
                      ? null
                      : (value) {
                          if (value != null) load(value);
                        },
                ),
                IconButton(
                  key: const Key('review-page-previous'),
                  tooltip: 'Previous change',
                  onPressed: !busy && index > 0
                      ? () => select(rows[index - 1])
                      : null,
                  icon: const Icon(Icons.chevron_left),
                ),
                Text(
                  rows.isEmpty
                      ? '0 changes'
                      : '${index + 1} of ${rows.length} changes',
                ),
                IconButton(
                  key: const Key('review-page-next'),
                  tooltip: 'Next change',
                  onPressed: !busy && index >= 0 && index < rows.length - 1
                      ? () => select(rows[index + 1])
                      : null,
                  icon: const Icon(Icons.chevron_right),
                ),
                const Text(
                  'Review view · changes are retained',
                  style: TextStyle(fontSize: 12),
                ),
                TextButton.icon(
                  key: const Key('review-page-close'),
                  onPressed: busy ? null : widget.close,
                  icon: const Icon(Icons.edit_outlined, size: 16),
                  label: const Text('Back to editing'),
                ),
              ],
            ),
          ),
        ),
        if (busy) const LinearProgressIndicator(),
        if (error != null)
          MaterialBanner(
            content: Text(error!, key: const Key('review-page-error')),
            actions: [
              TextButton(
                onPressed: busy ? null : () => load(mode),
                child: const Text('Retry'),
              ),
            ],
          ),
        Expanded(
          child: data == null
              ? const Center(child: Text('Preparing document review…'))
              : LayoutBuilder(
                  builder: (context, constraints) {
                    if (constraints.maxWidth < 780) {
                      return Column(
                        children: [
                          Expanded(child: page()),
                          SizedBox(
                            height: constraints.maxHeight * .38,
                            child: sidebar(),
                          ),
                        ],
                      );
                    }
                    return Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Expanded(child: page()),
                        const VerticalDivider(width: 1),
                        SizedBox(width: 280, child: sidebar()),
                      ],
                    );
                  },
                ),
        ),
      ],
    );
  }
}
