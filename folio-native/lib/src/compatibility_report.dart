// The Compatibility Report (W16.09, X17.09) and the impact preview before an
// export that would lose or invalidate something (D02.17). Both come from the
// engine's `compatibilityReport`, the table docs/status/COMPATIBILITY.md is
// generated from.
import 'package:flutter/material.dart';

/// The report's groups, in the order they're shown.
const compatibilityStates = [
  ('Verified', 'Edited and saved by Folio'),
  ('Partial', 'Partly edited; the rest is kept'),
  ('Preserved only', 'Kept unchanged, not edited'),
  ('Unsupported', 'Not kept or not written'),
];

/// What exporting to [format] will lose or invalidate.
List<Map> exportImpacts(Map result, String format) => [
  for (final report in (result['reports'] as List? ?? const []))
    if (report is Map && report['format'] == format)
      ...(report['impacts'] as List? ?? const []).whereType<Map>(),
];

class CompatibilityReportView extends StatelessWidget {
  final Map report;
  const CompatibilityReportView({required this.report, super.key});

  @override
  Widget build(BuildContext context) {
    final features = (report['features'] as List? ?? const [])
        .whereType<Map>()
        .toList();
    final impacts = (report['impacts'] as List? ?? const [])
        .whereType<Map>()
        .toList();
    final format = report['format'].toString().toUpperCase();
    final theme = Theme.of(context);
    if (report['created'] == true) {
      return Text(
        '$format · Created in Folio. Everything in it is written by Folio.',
        style: const TextStyle(fontWeight: FontWeight.w600),
      );
    }
    return Column(
      key: ValueKey('compatibility-report-${report['format']}'),
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          '$format · What this file contains',
          style: const TextStyle(fontWeight: FontWeight.w600),
        ),
        for (final impact in impacts)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(
              'On export: ${impact['message']}',
              style: TextStyle(color: theme.colorScheme.error),
            ),
          ),
        for (final (state, description) in compatibilityStates)
          if (features.any((f) => f['state'] == state)) ...[
            Padding(
              padding: const EdgeInsets.only(top: 12, bottom: 4),
              child: Semantics(
                header: true,
                child: Text(
                  '$state · $description',
                  style: theme.textTheme.titleSmall,
                ),
              ),
            ),
            for (final f in features.where((f) => f['state'] == state))
              Padding(
                padding: const EdgeInsets.only(top: 4, left: 8),
                child: Text(
                  '${f['name']} (${f['count']}): ${f['onSave']}',
                  key: ValueKey('compatibility-feature-${f['id']}'),
                ),
              ),
          ],
      ],
    );
  }
}

enum ExportChoice { export, folioCopy, cancel }

/// Lists what exporting will lose or invalidate, with the choice to export
/// anyway, save a Folio copy instead, or cancel.
Future<ExportChoice> showExportImpacts(
  BuildContext context,
  String format,
  List<Map> impacts,
) async {
  final choice = await showDialog<ExportChoice>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text('Saving as ${format.toUpperCase()} will change this file'),
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              for (final impact in impacts)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Text(
                    '• ${impact['message']}',
                    key: ValueKey('export-impact-${impact['id']}'),
                  ),
                ),
              const Text(
                'A Folio copy keeps everything as it is now, and you can export later.',
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, ExportChoice.cancel),
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, ExportChoice.folioCopy),
          child: const Text('Save Folio Copy…'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, ExportChoice.export),
          child: const Text('Export Anyway'),
        ),
      ],
    ),
  );
  return choice ?? ExportChoice.cancel;
}
