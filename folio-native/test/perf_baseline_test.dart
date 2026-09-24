// Edit round trip through the real bridge, including Dart JSON decoding.
// Opt-in: FOLIO_PERF=1 flutter test test/perf_baseline_test.dart
import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/core.dart';

double percentile(List<double> values, double p) {
  final sorted = [...values]..sort();
  return sorted[((sorted.length - 1) * p).round()];
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final core = CoreBridge();
  setUpAll(CoreBridge.initialize);
  test(
    'single-cell edit round trip on a 20-sheet, 100k-cell workbook',
    () async {
      const sheets = 20, rows = 2500, edits = 50;
      await core.call({'op': 'new', 'blank': true});
      final workspace = Map<String, dynamic>.from(
        (await core.call({'op': 'get'}))['workspace'],
      );
      workspace['sheets'] = [
        for (var s = 1; s <= sheets; s++)
          {
            'id': 's$s',
            'name': 'Sheet $s',
            'cells': {
              for (var r = 1; r <= rows; r++) ...{'A$r': '$r', 'B$r': '=A$r*2'},
            },
          },
      ];
      await core.call({'op': 'replace', 'workspace': workspace});
      final roundTrip = <double>[];
      Map<String, dynamic> last = {};
      for (var i = 0; i < edits; i++) {
        final watch = Stopwatch()..start();
        last = await core.call({
          'op': 'cells',
          'cells': [
            {'sheetId': 's1', 'address': 'A1', 'value': '${i + 1}'},
          ],
        });
        roundTrip.add(watch.elapsedMicroseconds / 1000);
      }
      expect(last['calculated']['s1']['B1'], edits * 2);
      // What the app does: WorkspaceModel.mutate stages the edit, measures
      // row heights, commits, and applies the delta responses.
      final folder = await Directory.systemTemp.createTemp('folio-perf');
      final model = WorkspaceModel(core, '${folder.path}/workspace.folio');
      model.snapshot = await core.call({'op': 'get'});
      final modelEdit = <double>[];
      for (var i = 0; i < edits; i++) {
        final watch = Stopwatch()..start();
        await model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 's1', 'address': 'A1', 'value': '${i + 7}'},
          ],
        });
        modelEdit.add(watch.elapsedMicroseconds / 1000);
      }
      expect(model.snapshot['calculated']['s1']['B1'], (edits + 6) * 2);
      model.dispose();
      await folder.delete(recursive: true);
      final report = {
        'workload': {
          'sheets': sheets,
          'populatedCells': sheets * rows * 2,
          'edits': edits,
        },
        // Direct `cells` call returning a full snapshot (the Phase 1 measure).
        'roundTripMs': {
          'p50': percentile(roundTrip, 0.5),
          'p95': percentile(roundTrip, 0.95),
        },
        'modelEditMs': {
          'p50': percentile(modelEdit, 0.5),
          'p95': percentile(modelEdit, 0.95),
        },
      };
      final out =
          Platform.environment['FOLIO_PERF_DART_OUT'] ??
          '${Directory.systemTemp.path}/folio-perf-dart.json';
      File(
        out,
      ).writeAsStringSync(const JsonEncoder.withIndent('  ').convert(report));
      // ignore: avoid_print
      print('${const JsonEncoder.withIndent('  ').convert(report)}\n$out');
    },
    skip: Platform.environment['FOLIO_PERF'] != '1',
    timeout: const Timeout(Duration(minutes: 10)),
  );
}
