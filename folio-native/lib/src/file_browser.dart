import 'dart:io';
import 'package:flutter/material.dart';
import 'recent_files.dart';
import 'working_files.dart';

class FileBrowserChoice {
  const FileBrowserChoice({this.recent, this.recovery, this.browse = false});
  final RecentFile? recent;
  final RecoveryCopy? recovery;
  final bool browse;
}

class FileBrowser extends StatefulWidget {
  const FileBrowser({
    super.key,
    required this.recent,
    required this.recovery,
    required this.inspect,
    this.historyError,
  });
  final RecentFiles recent;
  final RecoveryCatalog recovery;
  final Future<Map<String, dynamic>> Function(String) inspect;
  final String? historyError;
  @override
  State<FileBrowser> createState() => _FileBrowserState();
}

class _FileBrowserState extends State<FileBrowser> {
  List<RecoveryCopy> _copies = [];
  bool _recovery = false, _pinned = false, _busy = false, _loading = true;
  String _search = '';
  String? _error;
  WorkingFileKind? _kind;
  RecentFile? _recent;
  RecoveryCopy? _copy;
  Map<String, dynamic>? _preview;
  String? _previewError;
  bool _previewLoading = false;
  int _generation = 0;
  @override
  void initState() {
    super.initState();
    _loadCopies();
  }

  Future<void> _loadCopies() async {
    try {
      final copies = await widget.recovery.list();
      if (mounted) setState(() => _copies = copies);
    } catch (e) {
      if (mounted) {
        setState(() => _error = 'Recovery copies could not be listed: $e');
      }
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _change(Future<void> Function() action) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action();
    } catch (e) {
      if (mounted) setState(() => _error = 'The list could not be updated: $e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _selectCopy(RecoveryCopy copy) async {
    final generation = ++_generation;
    setState(() {
      _copy = copy;
      _preview = null;
      _previewError = null;
      _previewLoading = true;
    });
    try {
      final preview = await widget.inspect(copy.path);
      if (mounted && generation == _generation) {
        setState(() => _preview = preview);
      }
    } catch (e) {
      if (mounted && generation == _generation) {
        setState(() => _previewError = 'This copy could not be opened: $e');
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _previewLoading = false);
      }
    }
  }

  String _date(DateTime? date) {
    if (date == null) return 'Saved location';
    final d = date.toLocal();
    return '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')} ${d.hour.toString().padLeft(2, '0')}:${d.minute.toString().padLeft(2, '0')}';
  }

  bool _matches(String value) =>
      value.toLowerCase().contains(_search.toLowerCase());
  @override
  Widget build(BuildContext context) {
    final recent = widget.recent.entries
        .where(
          (e) =>
              (!_pinned || e.pinned) &&
              (_kind == null || e.kind == _kind) &&
              _matches(e.path),
        )
        .toList();
    final copies = _copies
        .where(
          (e) => _matches('${e.title} ${e.reason ?? ''} ${_date(e.modified)}'),
        )
        .toList();
    return AlertDialog(
      title: const Text('Recent files and recovery'),
      content: SizedBox(
        width: 800,
        height: 510,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SegmentedButton<bool>(
              segments: const [
                ButtonSegment(
                  value: false,
                  label: Text('Recent files'),
                  icon: Icon(Icons.history),
                ),
                ButtonSegment(
                  value: true,
                  label: Text('Recovery copies'),
                  icon: Icon(Icons.restore),
                ),
              ],
              selected: {_recovery},
              onSelectionChanged: _busy
                  ? null
                  : (v) => setState(() => _recovery = v.single),
            ),
            const SizedBox(height: 12),
            TextField(
              decoration: const InputDecoration(
                prefixIcon: Icon(Icons.search),
                labelText: 'Search files',
              ),
              onChanged: (v) => setState(() {
                _search = v;
                _recent = null;
                _copy = null;
                ++_generation;
                _preview = null;
                _previewLoading = false;
                _previewError = null;
              }),
            ),
            if (!_recovery)
              Row(
                children: [
                  DropdownButton<WorkingFileKind?>(
                    value: _kind,
                    items: [
                      const DropdownMenuItem(
                        value: null,
                        child: Text('All file types'),
                      ),
                      for (final kind in WorkingFileKind.values)
                        DropdownMenuItem(
                          value: kind,
                          child: Text(kind.name.toUpperCase()),
                        ),
                    ],
                    onChanged: (v) => setState(() {
                      _kind = v;
                      _recent = null;
                    }),
                  ),
                  const SizedBox(width: 16),
                  FilterChip(
                    label: const Text('Pinned'),
                    selected: _pinned,
                    onSelected: (v) => setState(() {
                      _pinned = v;
                      _recent = null;
                    }),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed:
                        _busy || !widget.recent.entries.any((e) => !e.pinned)
                        ? null
                        : () => _change(() async {
                            await widget.recent.clearUnpinned();
                            if (mounted) setState(() => _recent = null);
                          }),
                    child: const Text('Clear unpinned'),
                  ),
                ],
              )
            else
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 10),
                child: Text(
                  'Copies kept before New, Open, or closing without saving. Restore opens an unsaved workspace.',
                ),
              ),
            if (_error != null || (!_recovery && widget.historyError != null))
              Text(
                _error ?? widget.historyError!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            const Divider(height: 1),
            Expanded(
              child: _recovery
                  ? Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Expanded(
                          child: _loading
                              ? const Center(child: CircularProgressIndicator())
                              : copies.isEmpty
                              ? const Center(
                                  child: Text('No recovery copies found.'),
                                )
                              : ListView.separated(
                                  itemCount: copies.length,
                                  separatorBuilder: (_, _) =>
                                      const Divider(height: 1),
                                  itemBuilder: (context, i) {
                                    final copy = copies[i];
                                    return ListTile(
                                      key: ValueKey(copy.path),
                                      selected: _copy?.path == copy.path,
                                      title: Text(copy.title),
                                      subtitle: Text(
                                        '${_date(copy.modified)}\n${copy.reason ?? 'Saved recovery copy'}',
                                      ),
                                      onTap: () => _selectCopy(copy),
                                    );
                                  },
                                ),
                        ),
                        const VerticalDivider(),
                        Expanded(
                          child: Padding(
                            padding: const EdgeInsets.all(12),
                            child: _previewLoading
                                ? const Center(
                                    child: CircularProgressIndicator(),
                                  )
                                : _previewError != null
                                ? Text(_previewError!)
                                : _preview == null
                                ? const Center(
                                    child: Text('Select a copy to preview it.'),
                                  )
                                : SingleChildScrollView(
                                    child: Column(
                                      crossAxisAlignment:
                                          CrossAxisAlignment.start,
                                      children: [
                                        Text(
                                          _preview!['title'] as String,
                                          style: Theme.of(
                                            context,
                                          ).textTheme.titleMedium,
                                        ),
                                        const SizedBox(height: 8),
                                        Text(
                                          'Worksheets: ${(_preview!['sheets'] as List).join(', ')}',
                                        ),
                                        const Divider(),
                                        SelectableText(
                                          (_preview!['document'] as String)
                                                  .isEmpty
                                              ? 'No plain text to preview'
                                              : _preview!['document'] as String,
                                        ),
                                        const SizedBox(height: 12),
                                        const Text(
                                          'Document preview shows the first 2,000 characters. The full workspace is restored.',
                                        ),
                                      ],
                                    ),
                                  ),
                          ),
                        ),
                      ],
                    )
                  : recent.isEmpty
                  ? const Center(
                      child: Text(
                        'No matching recent files. Use Browse to open a file.',
                      ),
                    )
                  : ListView.separated(
                      itemCount: recent.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, i) {
                        final file = recent[i];
                        return ListTile(
                          key: ValueKey(file.key),
                          selected: _recent?.key == file.key,
                          leading: Icon(
                            file.kind == WorkingFileKind.xlsx
                                ? Icons.grid_on
                                : Icons.description_outlined,
                          ),
                          title: Text(file.name),
                          subtitle: Text(
                            '${File(file.path).parent.path}\n${_date(file.lastUsed)}',
                          ),
                          isThreeLine: true,
                          onTap: _busy
                              ? null
                              : () => setState(() => _recent = file),
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              IconButton(
                                tooltip: file.pinned
                                    ? 'Unpin ${file.name}'
                                    : 'Pin ${file.name}',
                                icon: Icon(
                                  file.pinned
                                      ? Icons.push_pin
                                      : Icons.push_pin_outlined,
                                ),
                                onPressed: _busy
                                    ? null
                                    : () => _change(
                                        () => widget.recent.pin(
                                          file,
                                          !file.pinned,
                                        ),
                                      ),
                              ),
                              IconButton(
                                tooltip: 'Remove ${file.name} from list',
                                icon: const Icon(Icons.close),
                                onPressed: _busy
                                    ? null
                                    : () => _change(() async {
                                        await widget.recent.remove(file);
                                        if (mounted &&
                                            _recent?.key == file.key) {
                                          setState(() => _recent = null);
                                        }
                                      }),
                              ),
                            ],
                          ),
                        );
                      },
                    ),
            ),
            if (!_recovery)
              const Padding(
                padding: EdgeInsets.only(top: 8),
                child: Text(
                  'Removing a file from this list does not delete it.',
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy
              ? null
              : () => Navigator.pop(
                  context,
                  const FileBrowserChoice(browse: true),
                ),
          child: const Text('Browse…'),
        ),
        TextButton(
          onPressed: _busy ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed:
              _busy ||
                  (_recovery
                      ? _copy == null || _preview == null || _previewLoading
                      : _recent == null)
              ? null
              : () => Navigator.pop(
                  context,
                  FileBrowserChoice(
                    recent: _recovery ? null : _recent,
                    recovery: _recovery ? _copy : null,
                  ),
                ),
          child: Text(_recovery ? 'Restore copy' : 'Open'),
        ),
      ],
    );
  }
}
