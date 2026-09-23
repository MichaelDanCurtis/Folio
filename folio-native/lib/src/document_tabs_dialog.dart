import 'document_paragraph_format.dart';
import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'document_styles.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'draft_text_field.dart';
import 'document_tabs.dart';
import 'document_tab_leaders.dart';

typedef TabStopsResult = ({List<DocumentTabStop> stops, int interval});

class DocumentTabsDialog extends StatefulWidget {
  final List<DocumentTabStop> stops;
  final int interval, paragraphCount;
  final Delta sample;
  final bool mixed, editDefault;
  const DocumentTabsDialog({
    required this.stops,
    required this.interval,
    required this.sample,
    required this.paragraphCount,
    this.mixed = false,
    this.editDefault = true,
    super.key,
  });
  @override
  State<DocumentTabsDialog> createState() => _DocumentTabsDialogState();
}

class _DocumentTabsDialogState extends State<DocumentTabsDialog> {
  late final stops = List<DocumentTabStop>.of(widget.stops);
  final position = TextEditingController();
  late final interval = TextEditingController(text: _points(widget.interval));
  String alignment = 'left', leader = 'none';
  int? selected;
  String? error;
  static const alignments = {
    'left': 'Left',
    'center': 'Center',
    'right': 'Right',
    'decimal': 'Decimal',
  };
  static String _points(int twips) =>
      (twips / 20).toStringAsFixed(twips % 20 == 0 ? 0 : 2);
  int? _twips(String s) {
    final n = double.tryParse(s.trim());
    return n != null && n.isFinite && n >= 0.05 && n <= 1584
        ? (n * 20).round()
        : null;
  }

  @override
  void dispose() {
    position.dispose();
    interval.dispose();
    super.dispose();
  }

  void setStop() {
    final p = _twips(position.text);
    if (p == null) {
      setState(
        () => error =
            'Enter a position greater than 0 and no more than 1,584 points.',
      );
      return;
    }
    setState(() {
      stops.removeWhere((s) => s.position == p || s.position == selected);
      stops.add(DocumentTabStop(p, alignment: alignment, leader: leader));
      stops.sort((a, b) => a.position.compareTo(b.position));
      selected = null;
      position.clear();
      error = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final spacing = _twips(interval.text);
    return AlertDialog(
      title: const Text('Tab stops'),
      content: SizedBox(
        width: 620,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                'Applies to ${widget.paragraphCount} selected ${widget.paragraphCount == 1 ? 'paragraph' : 'paragraphs'}.',
              ),
              if (widget.mixed)
                const Padding(
                  padding: EdgeInsets.only(top: 8),
                  child: Text(
                    'These paragraphs have different stops. Applying will use the stops shown here for all selected paragraphs.',
                  ),
                ),
              const SizedBox(height: 12),
              Container(
                constraints: const BoxConstraints(
                  maxHeight: 180,
                  minHeight: 64,
                ),
                decoration: BoxDecoration(
                  border: Border.all(color: const Color(0xffdfe3e8)),
                ),
                child: stops.isEmpty
                    ? const Center(
                        child: Padding(
                          padding: EdgeInsets.all(16),
                          child: Text(
                            'No custom stops. Tabs use the default spacing.',
                          ),
                        ),
                      )
                    : ListView(
                        shrinkWrap: true,
                        children: [
                          for (final s in stops)
                            ListTile(
                              dense: true,
                              selected: selected == s.position,
                              key: ValueKey('tab-stop-${s.position}'),
                              title: Text('${_points(s.position)} pt'),
                              subtitle: Text(
                                '${alignments[s.alignment] ?? s.alignment}${s.leader == 'none' ? '' : ' · ${tabLeaderNames[s.leader] ?? s.leader}'}',
                              ),
                              onTap: () => setState(() {
                                selected = s.position;
                                position.text = _points(s.position);
                                alignment = alignments.containsKey(s.alignment)
                                    ? s.alignment
                                    : 'left';
                                leader = s.leader;
                                error = null;
                              }),
                              trailing: IconButton(
                                tooltip:
                                    'Remove ${_points(s.position)} point stop',
                                icon: const Icon(Icons.close, size: 18),
                                onPressed: () => setState(() {
                                  stops.remove(s);
                                  if (selected == s.position) {
                                    selected = null;
                                    position.clear();
                                  }
                                }),
                              ),
                            ),
                        ],
                      ),
              ),
              const SizedBox(height: 12),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: draftTextField(
                      key: const Key('tab-position'),
                      controller: position,
                      decoration: const InputDecoration(
                        labelText: 'Position (points)',
                        helperText: '72 points = 1 inch',
                      ),
                      onSubmitted: (_) => setStop(),
                      onChanged: (_) => setState(() => error = null),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: DropdownButtonFormField<String>(
                      isExpanded: true,
                      key: ValueKey('tab-alignment-$alignment'),
                      initialValue: alignment,
                      decoration: const InputDecoration(labelText: 'Alignment'),
                      items: [
                        for (final a in alignments.entries)
                          DropdownMenuItem(value: a.key, child: Text(a.value)),
                      ],
                      onChanged: (v) => setState(() => alignment = v!),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: DropdownButtonFormField<String>(
                      isExpanded: true,
                      key: ValueKey('tab-leader-$leader'),
                      initialValue: leader,
                      decoration: const InputDecoration(labelText: 'Leader'),
                      items: [
                        for (final l in tabLeaderNames.entries)
                          DropdownMenuItem(value: l.key, child: Text(l.value)),
                        if (!tabLeaderNames.containsKey(leader))
                          DropdownMenuItem(
                            value: leader,
                            child: Text('Imported: $leader'),
                          ),
                      ],
                      onChanged: (value) => setState(() => leader = value!),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Padding(
                    padding: const EdgeInsets.only(top: 12),
                    child: TextButton(
                      onPressed: setStop,
                      child: Text(
                        selected == null ? 'Add stop' : 'Update stop',
                      ),
                    ),
                  ),
                ],
              ),
              if (position.text.trim().isNotEmpty)
                const Text('Use Add stop or Update stop before applying.'),
              if (error != null)
                Text(
                  error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              Align(
                alignment: Alignment.centerLeft,
                child: TextButton(
                  onPressed: stops.isEmpty
                      ? null
                      : () => setState(() {
                          stops.clear();
                          selected = null;
                          position.clear();
                        }),
                  child: const Text('Clear all stops'),
                ),
              ),
              if (widget.editDefault)
                draftTextField(
                  key: const Key('default-tab-spacing'),
                  controller: interval,
                  decoration: InputDecoration(
                    labelText: 'Default spacing (points)',
                    helperText:
                        'Applies throughout the document, after the last custom stop.',
                    errorText: spacing == null
                        ? 'Enter a spacing from 0.05 to 1,584 points.'
                        : null,
                  ),
                  onChanged: (_) => setState(() {}),
                ),
              const SizedBox(height: 20),
              const Text(
                'Preview',
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.all(12),
                color: Colors.white,
                child: _TabParagraphPreview(
                  key: ValueKey(
                    jsonEncode([
                      stops.map((s) => s.toJson()).toList(),
                      spacing,
                    ]),
                  ),
                  sample: widget.sample,
                  stops: stops,
                  interval: spacing ?? widget.interval,
                ),
              ),
              if (stops.any(
                (s) =>
                    !alignments.containsKey(s.alignment) ||
                    !tabLeaderNames.containsKey(s.leader),
              ))
                const Padding(
                  padding: EdgeInsets.only(top: 8),
                  child: Text(
                    'Some imported tab types are preserved in the Word file. Their preview is not yet complete.',
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('apply-tab-stops'),
          onPressed: spacing == null || position.text.trim().isNotEmpty
              ? null
              : () => Navigator.pop(context, (
                  stops: List<DocumentTabStop>.of(stops),
                  interval: spacing,
                )),
          child: const Text('Apply'),
        ),
      ],
    );
  }
}

class _TabParagraphPreview extends StatefulWidget {
  final Delta sample;
  final List<DocumentTabStop> stops;
  final int interval;
  const _TabParagraphPreview({
    required this.sample,
    required this.stops,
    required this.interval,
    super.key,
  });
  @override
  State<_TabParagraphPreview> createState() => _TabParagraphPreviewState();
}

class _TabParagraphPreviewState extends State<_TabParagraphPreview> {
  late final controller = q.QuillController(
    document: q.Document.fromDelta(
      widget.sample.compose(
        changeTabStops(
          widget.sample,
          const TextSelection.collapsed(offset: 0),
          widget.stops,
        ),
      ),
    ),
    selection: const TextSelection.collapsed(offset: 0),
    readOnly: true,
  );
  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: DocumentTabSettings(
      intervalTwips: widget.interval,
      child: q.QuillParagraphLayout(
        builder: layoutDocumentTabs,
        paddingBuilder: paragraphPadding,
        styleBuilder: paragraphTextStyle,
        strutBuilder: paragraphStrut,
        blockSpacingBuilder: paragraphBlockSpacing,
        child: q.QuillEditor.basic(
          key: const Key('tab-stops-preview'),
          controller: controller,
          config: q.QuillEditorConfig(
            scrollable: false,
            showCursor: false,
            enableInteractiveSelection: false,
            enableSelectionToolbar: false,
            customStyles: documentEditorStyles(),
          ),
        ),
      ),
    ),
  );
}
