// Delta responses from the engine, applied by the model, must reproduce the
// engine's full snapshots without disturbing earlier snapshots.
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/core.dart';

Map<String, dynamic> _content(Map<String, dynamic> s) =>
    Map<String, dynamic>.of(s)
      ..remove('fileStates')
      ..remove('epoch');

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final core = CoreBridge();
  setUpAll(CoreBridge.initialize);

  Future<void> load() async {
    await core.call({'op': 'new', 'blank': true});
    final workspace = Map<String, dynamic>.from(
      (await core.call({'op': 'get'}))['workspace'],
    );
    workspace['sheets'] = [
      {
        'id': 'a',
        'name': 'A',
        'cells': {
          for (var r = 1; r <= 12; r++) ...{'A$r': '$r', 'B$r': '=A$r*2'},
        },
        'conditionalFormats': [
          {
            'sqref': 'A1:B12',
            'type': 'cellIs',
            'operator': 'greaterThan',
            'formulas': ['10'],
            'priority': 1,
            'style': {'bold': true},
          },
        ],
      },
      {
        'id': 'b',
        'name': 'B',
        'cells': {'A1': "='A'!B2"},
      },
    ];
    await core.call({'op': 'replace', 'workspace': workspace});
  }

  test(
    'engine deltas applied to the held state equal full snapshots',
    () async {
      await load();
      var held = await core.call({'op': 'get'});
      final random = Random(7);
      const values = ['', '4', '15', '=A1+A2', '=1/0', 'text'];
      var deltas = 0;
      for (var step = 0; step < 120; step++) {
        final kind = random.nextInt(6);
        final request = switch (kind) {
          0 => {'op': 'undo'},
          1 => {'op': 'redo'},
          2 => {
            'op': 'formatCells',
            'sheetId': 'a',
            'addresses': ['A${1 + random.nextInt(5)}'],
            'style': {'numFmt': '0.0'},
          },
          _ => {
            'op': 'cells',
            'cells': [
              {
                'sheetId': ['a', 'b'][random.nextInt(2)],
                'address':
                    '${'ABC'[random.nextInt(3)]}${1 + random.nextInt(8)}',
                'value': values[random.nextInt(values.length)],
              },
            ],
          },
        };
        final before = jsonEncode(held);
        final response = await core.call({...request, 'delta': held['epoch']});
        if (response.containsKey('delta')) deltas++;
        final next = applyEngineDelta(held, response);
        expect(jsonEncode(held), before, reason: 'step $step changed the base');
        final full = await core.call({'op': 'get'});
        expect(
          _content(next)..remove('documentHistoryChange'),
          _content(full),
          reason: 'step $step: $request',
        );
        held = next;
      }
      expect(deltas, greaterThan(50));
    },
  );

  testWidgets('the model adopts deltas and refreshes file states', (
    tester,
  ) async {
    await tester.runAsync(() async {
      final folder = await Directory.systemTemp.createTemp('folio-delta');
      addTearDown(() => folder.delete(recursive: true));
      await load();
      final model = WorkspaceModel(core, '${folder.path}/workspace.folio');
      addTearDown(model.dispose);
      model.snapshot = await core.call({'op': 'get'});
      model.rowMeasurer = (prepared) async => {
        for (final MapEntry(:key, :value)
            in (prepared['layoutRows'] as Map).entries)
          key: {for (final row in value as List) '$row': 20},
      };
      for (var i = 0; i < 12; i++) {
        final previous = jsonEncode(model.snapshot);
        final held = model.snapshot;
        await model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'a', 'address': 'A${1 + i % 4}', 'value': '${i * 3}'},
          ],
        });
        expect(jsonEncode(held), previous, reason: 'edit $i changed the base');
        final full = await core.call({'op': 'get'});
        expect(_content(model.snapshot), _content(full), reason: 'edit $i');
        await model.flush();
        expect(model.snapshot['fileStates'], full['fileStates']);
      }
      await model.mutate({'op': 'undo'});
      final full = await core.call({'op': 'get'});
      expect(
        _content(model.snapshot)..remove('documentHistoryChange'),
        _content(full),
      );
      expect(await model.currentFileStates(), full['fileStates']);
    });
  });
}
