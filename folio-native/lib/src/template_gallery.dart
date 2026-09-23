import 'package:flutter/material.dart';
import 'templates.dart';
import 'working_files.dart';

class TemplateGallery extends StatefulWidget {
  const TemplateGallery({
    super.key,
    required this.library,
    required this.preview,
    required this.browse,
    this.libraryError,
  });
  final TemplateLibrary library;
  final Future<Map<String, dynamic>> Function(OfficeTemplate) preview;
  final Future<OfficeTemplate?> Function(OfficeTemplate?) browse;
  final String? libraryError;
  @override
  State<TemplateGallery> createState() => _TemplateGalleryState();
}

class _TemplateGalleryState extends State<TemplateGallery> {
  String _search = '';
  final _searchController = TextEditingController();
  WorkingFileKind? _kind;
  OfficeTemplate? _selected;
  Map<String, dynamic>? _preview;
  String? _error;
  bool _loading = false, _busy = false;
  int _generation = 0;
  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> select(OfficeTemplate template) async {
    final generation = ++_generation;
    setState(() {
      _selected = template;
      _preview = null;
      _error = null;
      _loading = true;
    });
    try {
      final preview = await widget.preview(template);
      if (mounted && generation == _generation) {
        setState(() => _preview = preview);
      }
    } catch (e) {
      if (mounted && generation == _generation) setState(() => _error = '$e');
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _loading = false);
      }
    }
  }

  Future<void> browse([OfficeTemplate? previous]) async {
    setState(() => _busy = true);
    try {
      final chosen = await widget.browse(previous);
      if (chosen != null && mounted) {
        setState(() {
          _search = '';
          _searchController.clear();
          _kind = null;
        });
        await select(chosen);
      }
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void clearSelection() {
    ++_generation;
    _selected = null;
    _preview = null;
    _error = null;
    _loading = false;
  }

  @override
  Widget build(BuildContext context) {
    final entries = widget.library.entries
        .where(
          (e) =>
              (_kind == null || e.kind == _kind) &&
              '${e.title} ${e.description}'.toLowerCase().contains(
                _search.toLowerCase(),
              ),
        )
        .toList();
    final ready = !_busy && !_loading && _preview != null && _selected != null;
    return AlertDialog(
      title: const Text('New from template'),
      content: SizedBox(
        width: 860,
        height: 530,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _searchController,
                    decoration: const InputDecoration(
                      labelText: 'Search templates',
                      prefixIcon: Icon(Icons.search),
                    ),
                    onChanged: (value) => setState(() {
                      _search = value;
                      clearSelection();
                    }),
                  ),
                ),
                const SizedBox(width: 16),
                DropdownButton<WorkingFileKind?>(
                  value: _kind,
                  items: const [
                    DropdownMenuItem(
                      value: null,
                      child: Text('Word and Excel'),
                    ),
                    DropdownMenuItem(
                      value: WorkingFileKind.docx,
                      child: Text('Word'),
                    ),
                    DropdownMenuItem(
                      value: WorkingFileKind.xlsx,
                      child: Text('Excel'),
                    ),
                  ],
                  onChanged: (v) => setState(() {
                    _kind = v;
                    clearSelection();
                  }),
                ),
              ],
            ),
            const SizedBox(height: 12),
            if (widget.libraryError != null)
              Text(
                widget.libraryError!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    child: entries.isEmpty
                        ? const Center(child: Text('No matching templates.'))
                        : ListView.separated(
                            itemCount: entries.length,
                            separatorBuilder: (_, _) =>
                                const Divider(height: 1),
                            itemBuilder: (context, i) {
                              final entry = entries[i];
                              return ListTile(
                                key: ValueKey('template-${entry.id}'),
                                selected: _selected?.id == entry.id,
                                leading: Icon(
                                  entry.kind == WorkingFileKind.docx
                                      ? Icons.description_outlined
                                      : Icons.grid_on,
                                  color: entry.kind == WorkingFileKind.docx
                                      ? const Color(0xff185abd)
                                      : const Color(0xff107c41),
                                ),
                                title: Text(entry.title),
                                subtitle: Text(
                                  entry.file == null
                                      ? entry.description
                                      : 'My templates · ${entry.description}',
                                  maxLines: 3,
                                  overflow: TextOverflow.ellipsis,
                                ),
                                onTap: _busy ? null : () => select(entry),
                              );
                            },
                          ),
                  ),
                  const VerticalDivider(),
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.all(12),
                      child: _loading
                          ? const Center(child: CircularProgressIndicator())
                          : _error != null
                          ? SingleChildScrollView(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    'This template could not be previewed.\n\n$_error',
                                  ),
                                  if (_selected?.file != null)
                                    TextButton(
                                      onPressed: _busy
                                          ? null
                                          : () => browse(_selected),
                                      child: const Text('Locate template…'),
                                    ),
                                ],
                              ),
                            )
                          : _preview == null
                          ? const Center(
                              child: Text(
                                'Choose a template to preview its contents.',
                              ),
                            )
                          : SingleChildScrollView(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    _selected!.title,
                                    style: Theme.of(
                                      context,
                                    ).textTheme.titleLarge,
                                  ),
                                  const SizedBox(height: 8),
                                  Text(
                                    _selected!.file == null
                                        ? _selected!.description
                                        : 'Your template',
                                  ),
                                  const Divider(),
                                  if (_selected!.kind == WorkingFileKind.docx)
                                    SelectableText(
                                      (_preview!['document'] as String).isEmpty
                                          ? 'No plain text to preview.'
                                          : _preview!['document'] as String,
                                    )
                                  else ...[
                                    Text(
                                      'Worksheets: ${(_preview!['sheets'] as List).join(', ')}',
                                    ),
                                    const SizedBox(height: 12),
                                    if (_preview!['sample'] is List)
                                      Table(
                                        border: TableBorder.all(
                                          color: Colors.black12,
                                        ),
                                        children: [
                                          for (final row
                                              in _preview!['sample'] as List)
                                            TableRow(
                                              children: [
                                                for (final cell in row as List)
                                                  Padding(
                                                    padding:
                                                        const EdgeInsets.all(5),
                                                    child: Text(
                                                      '$cell',
                                                      maxLines: 3,
                                                      overflow:
                                                          TextOverflow.ellipsis,
                                                      style: const TextStyle(
                                                        fontSize: 11,
                                                      ),
                                                    ),
                                                  ),
                                              ],
                                            ),
                                        ],
                                      ),
                                  ],
                                  const SizedBox(height: 16),
                                  const Text(
                                    'Content preview only. Create opens a new file; the original template stays unchanged.',
                                  ),
                                ],
                              ),
                            ),
                    ),
                  ),
                ],
              ),
            ),
            if (_selected?.file != null)
              Align(
                alignment: Alignment.centerLeft,
                child: TextButton(
                  onPressed: _busy
                      ? null
                      : () async {
                          setState(() => _busy = true);
                          try {
                            await widget.library.files.remove(_selected!.file!);
                            if (mounted) setState(clearSelection);
                          } catch (e) {
                            if (mounted) setState(() => _error = '$e');
                          } finally {
                            if (mounted) setState(() => _busy = false);
                          }
                        },
                  child: const Text('Remove from library (keep file)'),
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => browse(),
          child: const Text('Browse templates…'),
        ),
        TextButton(
          onPressed: _busy ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        if (_selected?.file != null)
          OutlinedButton(
            onPressed: ready
                ? () => Navigator.pop(
                    context,
                    TemplateSelection(
                      _selected!,
                      _preview!['fingerprint'] as String,
                      edit: true,
                    ),
                  )
                : null,
            child: const Text('Edit template'),
          ),
        FilledButton(
          onPressed: ready
              ? () => Navigator.pop(
                  context,
                  TemplateSelection(
                    _selected!,
                    _preview!['fingerprint'] as String,
                  ),
                )
              : null,
          child: const Text('Create'),
        ),
      ],
    );
  }
}
