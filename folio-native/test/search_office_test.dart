import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/search.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'replacements survive Office export and workspace reopen with atomic history',
    () async {
      await CoreBridge.initialize();
      final target = Platform.environment['FOLIO_SEARCH_OUTPUT'];
      final dir = target == null
          ? await Directory.systemTemp.createTemp('folio-search-office-')
          : await Directory(target).create(recursive: true);
      if (target == null) addTearDown(() => dir.delete(recursive: true));
      final core = CoreBridge();
      await core.call({'op': 'new'});
      await core.call({
        'op': 'importOffice',
        'path': File('test/fixtures/source.docx').absolute.path,
      });
      var state = await core.call({
        'op': 'importOffice',
        'path': File('test/fixtures/source.xlsx').absolute.path,
      });
      final original = state['workspace'];
      final index = DocumentSearchIndex(original['document']);
      final hits = index.find('Revenue', const SearchOptions()).hits;
      expect(hits.length, 2); // Body paragraph and table header.
      final document = index.replace(original['document'], hits, 'Income');
      final cells = findInWorkbook(
        original['sheets'],
        state['calculated'],
        'office-1',
        '2400',
        const SearchOptions(allSheets: true, entireCell: true),
        formatted: state['formatted'] as Map,
      ).hits;
      final edits = workbookReplacements(
        original['sheets'],
        cells,
        '2400',
        '3000',
        const SearchOptions(allSheets: true, entireCell: true),
      );
      final proposal = {
        'baseRevision': original['revision'],
        '_workspaceId': original['id'],
        'summary': 'Replace in imported document and workbook',
        'document': document,
        'cells': edits,
      };
      state = await core.call({'op': 'apply', 'proposal': proposal});
      expect(state['calculated']['office-1']['B4'], 10600);
      expect(state['calculated']['office-1']['AA1'], 6000);
      final changed = state['workspace'];
      await expectLater(
        core.call({'op': 'apply', 'proposal': proposal}),
        throwsStateError,
      );
      expect((await core.call({'op': 'get'}))['workspace'], changed);
      await core.call({'op': 'save', 'path': '${dir.path}/replaced.folio'});
      final undo = await core.call({'op': 'undo'});
      expect(undo['workspace']['document'], original['document']);
      expect(undo['workspace']['sheets'], original['sheets']);
      await core.call({'op': 'new'});
      state = await core.call({
        'op': 'load',
        'path': '${dir.path}/replaced.folio',
      });
      expect(state['workspace']['document'], changed['document']);
      expect(state['workspace']['sheets'], changed['sheets']);
      for (final format in ['docx', 'xlsx']) {
        await core.call({
          'op': 'exportOffice',
          'format': format,
          'path': '${dir.path}/replaced.$format',
        });
      }
      state = await core.call({
        'op': 'importOffice',
        'path': '${dir.path}/replaced.docx',
      });
      expect(
        DocumentSearchIndex(
          state['workspace']['document'],
        ).find('Income', const SearchOptions()).hits.length,
        2,
      );
      state = await core.call({
        'op': 'importOffice',
        'path': '${dir.path}/replaced.xlsx',
      });
      expect(state['workspace']['sheets'][0]['cells']['B2'], '3000');
      expect(state['workspace']['sheets'][0]['cells']['B4'], '=SUM(B2:B3)');
    },
  );
}
