import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/core.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final core = CoreBridge();
  setUpAll(CoreBridge.initialize);
  setUp(() async {
    await core.call({'op': 'new'});
  });
  test(
    'native bridge stages and atomically applies document and sheet edits',
    () async {
      final before = await core.call({'op': 'get'});
      final p = await core.call({
        'op': 'propose',
        'proposal': {
          'summary': 'Change both',
          'baseRevision': before['workspace']['revision'],
          'appendParagraphs': ['Added natively.'],
          'cells': [
            {'sheetId': 'budget', 'address': 'B4', 'value': '3000'},
          ],
        },
      });
      expect(
        (await core.call({'op': 'get'}))['workspace'],
        before['workspace'],
      );
      final after = await core.call({'op': 'apply', 'proposal': p});
      expect(after['calculated']['budget']['B9'], 20600);
      expect(
        after['workspace']['document']['content'].last['content'][0]['text'],
        'Added natively.',
      );
      await expectLater(
        core.call({'op': 'apply', 'proposal': p}),
        throwsStateError,
      );
      final undone = await core.call({'op': 'undo'});
      expect(undone['workspace']['document'], before['workspace']['document']);
      expect(undone['calculated']['budget']['B9'], 20000);
      final redone = await core.call({'op': 'redo'});
      expect(redone['calculated']['budget']['B9'], 20600);
    },
  );
  test('queued edits save and reopen without losing a change', () async {
    final dir = await Directory.systemTemp.createTemp('folio-native-test-');
    addTearDown(() => dir.delete(recursive: true));
    final model = WorkspaceModel(core, '${dir.path}/workspace.folio');
    addTearDown(model.dispose);
    await model.initialize();
    await Future.wait([
      model.mutate({
        'op': 'cells',
        'cells': [
          {'sheetId': 'budget', 'address': 'B4', 'value': '3200'},
        ],
      }),
      model.mutate({'op': 'title', 'title': 'Persistence verified'}),
    ]);
    await model.flush();
    await model.saveCopy('${dir.path}/copy.folio');
    await core.call({'op': 'new'});
    await model.open('${dir.path}/copy.folio');
    expect(model.workspace['title'], 'Persistence verified');
    expect(model.workspace['sheets'][0]['cells']['B4'], '3200');
  });
  test(
    'damaged recovery bytes are preserved before new work is saved',
    () async {
      final dir = await Directory.systemTemp.createTemp('folio-recovery-test-');
      addTearDown(() => dir.delete(recursive: true));
      final file = File('${dir.path}/workspace.folio');
      await file.writeAsString('{broken');
      final model = WorkspaceModel(core, file.path);
      addTearDown(model.dispose);
      await model.initialize();
      await model.flush();
      final backup = dir.listSync().whereType<File>().firstWhere(
        (f) => f.path.contains('.unreadable-'),
      );
      expect(await backup.readAsString(), '{broken');
      expect(model.error, contains('preserved'));
    },
  );
  test('CSV keeps zeros in decimal and integer values', () async {
    await core.call({
      'op': 'cells',
      'cells': [
        {'sheetId': 'budget', 'address': 'E1', 'value': '1000'},
        {'sheetId': 'budget', 'address': 'E2', 'value': '10.01'},
      ],
    });
    final csv = await core.call({'op': 'exportCsv', 'sheetId': 'budget'});
    expect(csv['text'], contains('1000'));
    expect(csv['text'], contains('10.01'));
  });
}
