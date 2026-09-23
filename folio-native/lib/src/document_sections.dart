import 'package:flutter/material.dart';

const sectionBreakNames = <String, String>{
  'nextPage': 'Next page',
  'continuous': 'Continuous',
  'evenPage': 'Even page',
  'oddPage': 'Odd page',
};

class DocumentSectionBreakDialog extends StatefulWidget {
  final Map data;
  const DocumentSectionBreakDialog({super.key, required this.data});
  @override
  State<DocumentSectionBreakDialog> createState() => _SectionBreakState();
}

class _SectionBreakState extends State<DocumentSectionBreakDialog> {
  String action = 'insert', start = 'nextPage';
  late int section;
  List get sections => widget.data['sections'] as List? ?? [];
  @override
  void initState() {
    super.initState();
    section = ((widget.data['targetSection'] as int? ?? 1) - 1).clamp(
      0,
      sections.length > 1 ? sections.length - 2 : 0,
    );
    if (widget.data['canInsert'] != true && sections.length > 1) {
      action = 'remove';
    }
  }

  String? get reason {
    if (widget.data['protected'] == true) {
      return 'This document is protected. Section breaks cannot be changed.';
    }
    if (widget.data['needsAuthor'] == true) {
      return 'Choose a review author in Recording settings before editing this document.';
    }
    if (widget.data['recordingEnabled'] == true) {
      return 'Section-break recording is not supported yet. Turn recording off explicitly in Review to make an unrecorded change.';
    }
    if (action == 'insert') {
      if (widget.data['canInsert'] != true) {
        return widget.data['insertReason'] as String? ??
            'Place the insertion point in a document paragraph.';
      }
      final target = widget.data['targetSection'] as int? ?? 0;
      if (sections[target]['hasTrackedSettings'] == true) {
        return 'Review this section’s pending formatting change before inserting a break.';
      }
    } else {
      if (sections.length < 2) {
        return 'This document has no section break to remove.';
      }
      if (sections[section]['hasTrackedSettings'] == true ||
          sections[section + 1]['hasTrackedSettings'] == true) {
        return 'Review these sections’ pending formatting changes before removing the break.';
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Section breaks'),
    content: SizedBox(
      width: 540,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SegmentedButton<String>(
              segments: const [
                ButtonSegment(
                  value: 'insert',
                  label: Text('Insert'),
                  icon: Icon(Icons.add),
                ),
                ButtonSegment(
                  value: 'remove',
                  label: Text('Remove'),
                  icon: Icon(Icons.remove),
                ),
              ],
              selected: {action},
              onSelectionChanged: (value) =>
                  setState(() => action = value.first),
            ),
            const SizedBox(height: 18),
            if (action == 'insert') ...[
              const Text(
                'Start a new section at the insertion point. Text after the break keeps its current formatting; its headers and footers link to the previous section.',
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                key: const Key('section-break-type'),
                initialValue: start,
                decoration: const InputDecoration(
                  labelText: 'New section starts',
                ),
                items: sectionBreakNames.entries
                    .map(
                      (e) =>
                          DropdownMenuItem(value: e.key, child: Text(e.value)),
                    )
                    .toList(),
                onChanged: (value) => setState(() => start = value!),
              ),
            ] else if (sections.length > 1) ...[
              DropdownButtonFormField<int>(
                key: const Key('section-break-boundary'),
                initialValue: section,
                decoration: const InputDecoration(labelText: 'Break to remove'),
                items: List.generate(
                  sections.length - 1,
                  (i) => DropdownMenuItem(
                    value: i,
                    child: Text('Between sections ${i + 1} and ${i + 2}'),
                  ),
                ),
                onChanged: (value) => setState(() => section = value!),
              ),
              const SizedBox(height: 12),
              Text(
                'The combined section will use Section ${section + 2}’s page settings. Adjacent text paragraphs will join. You can restore the break with Undo.',
              ),
            ],
            const SizedBox(height: 16),
            const Text(
              'The editor currently shows a continuous view. Page separation is stored in the Word file; complete on-screen pagination is still in development.',
              style: TextStyle(fontSize: 12),
            ),
            if (reason != null) ...[
              const SizedBox(height: 14),
              Text(
                reason!,
                key: const Key('section-break-reason'),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        key: const Key('section-break-cancel'),
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('section-break-apply'),
        onPressed: reason == null
            ? () => Navigator.pop(context, <String, dynamic>{
                'action': action,
                if (action == 'insert') ...{
                  'start': start,
                  'targetKey': widget.data['targetKey'],
                },
                if (action == 'remove') ...{
                  'section': section,
                  'sectionKey': sections[section]['key'],
                },
              })
            : null,
        child: Text(action == 'insert' ? 'Insert break' : 'Remove break'),
      ),
    ],
  );
}
