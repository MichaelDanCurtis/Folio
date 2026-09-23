import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/recent_files.dart';
import 'package:folio_native/src/working_files.dart';

void main() {
  late Directory dir;
  setUp(
    () async =>
        dir = await Directory.systemTemp.createTemp('folio-recent-test-'),
  );
  tearDown(() => dir.delete(recursive: true));

  test(
    'migration survives session reset, empty history stays empty, grants stay local',
    () async {
      final legacy = File('${dir.path}/working-files.json');
      await legacy.writeAsString(
        jsonEncode({
          'workspaceId': 'old',
          'files': [
            {'kind': 'docx', 'path': '/old/report.docx', 'bookmark': 'grant'},
          ],
        }),
      );
      final recent = RecentFiles('${dir.path}/recent.json');
      await recent.load(migrateFrom: legacy.path);
      expect(recent.entries.single.bookmark, 'grant');
      expect(recent.entries.single.lastUsed, isNull);
      await WorkingFiles(legacy.path).load(
        'blank',
        workspace: {'id': 'blank'},
        fileStates: {},
        pristine: true,
      );
      await recent.remove(recent.entries.single);
      final loaded = RecentFiles(recent.storePath);
      await loaded.load(migrateFrom: legacy.path);
      expect(loaded.entries, isEmpty);
      expect(jsonDecode(await legacy.readAsString())['files'], isEmpty);
    },
  );

  test(
    'serialized updates keep pins through rename and cap only unpinned history',
    () async {
      final recent = RecentFiles('${dir.path}/recent.json');
      await recent.load();
      final actual = File('${dir.path}/original.docx');
      await actual.writeAsString('Do not delete');
      await recent.remember(WorkingFileKind.docx, actual.path, 'old-grant');
      await recent.pin(recent.entries.single, true);
      await Future.wait([
        for (var i = 0; i < 60; i++)
          recent.remember(WorkingFileKind.xlsx, '/files/$i.xlsx', 'grant-$i'),
      ]);
      expect(recent.entries, hasLength(51));
      expect(recent.entries.first.path, actual.path);
      await recent.remember(
        WorkingFileKind.docx,
        '/renamed.docx',
        'new-grant',
        previousPath: actual.path,
      );
      expect(recent.entries.first.pinned, isTrue);
      expect(recent.entries.first.bookmark, 'new-grant');
      expect(recent.entries.any((e) => e.path == actual.path), isFalse);
      await recent.clearUnpinned();
      expect(recent.entries, hasLength(1));
      final loaded = RecentFiles(recent.storePath);
      await loaded.load();
      await loaded.remove(loaded.entries.single);
      expect(await actual.readAsString(), 'Do not delete');
    },
  );

  test(
    'bad metadata is preserved and failed writes do not publish changes',
    () async {
      final file = File('${dir.path}/recent.json');
      await file.writeAsString('{bad');
      final recent = RecentFiles(file.path);
      await expectLater(recent.load(), throwsFormatException);
      await expectLater(
        recent.remember(WorkingFileKind.docx, '/a.docx', 'a'),
        throwsStateError,
      );
      expect(await file.readAsString(), '{bad');
      await file.delete();
      await recent.load();
      await Directory('${file.path}.tmp').create();
      await expectLater(
        recent.remember(WorkingFileKind.docx, '/a.docx', 'a'),
        throwsA(isA<FileSystemException>()),
      );
      expect(recent.entries, isEmpty);
      await Directory('${file.path}.tmp').delete();
      await recent.remember(WorkingFileKind.docx, '/a.docx', 'a');
      expect(recent.entries, hasLength(1));
    },
  );

  test(
    'snapshots have unique locations and legacy copies remain discoverable',
    () async {
      final catalog = RecoveryCatalog('${dir.path}/Snapshots');
      final paths = await Future.wait([
        for (var i = 0; i < 10; i++) catalog.reserve(),
      ]);
      expect(paths.toSet(), hasLength(10));
      for (final path in paths) {
        await File(path).writeAsString('{}');
      }
      await catalog.describe(paths.first, {
        'description': {'title': 'Kept title'},
        'fingerprint': 'sha',
      }, 'Before a new workspace');
      await File('${catalog.directory}/legacy.folio').writeAsString('{}');
      await File('${paths.last}.json').writeAsString('{broken');
      await Link('${catalog.directory}/linked.folio').create(paths.first);
      final copies = await catalog.list();
      expect(copies, hasLength(11));
      expect(copies.any((e) => e.title == 'Kept title'), isTrue);
      expect(copies.any((e) => e.path.endsWith('/legacy.folio')), isTrue);
      expect(copies.any((e) => e.path.endsWith('/linked.folio')), isFalse);
    },
  );

  test(
    'recovered drafts ask to save the entire workspace until bound',
    () async {
      final files = WorkingFiles('${dir.path}/working.json');
      await files.reset('recovered', asDraft: true);
      final loaded = WorkingFiles(files.storePath);
      await loaded.load('recovered');
      expect(loaded.workspaceDraft, isTrue);
      expect(
        loaded
            .pending(
              {'id': 'recovered'},
              {},
              0,
              replacing: WorkingFileKind.xlsx,
            )
            .single
            .kind,
        WorkingFileKind.folio,
      );
      await loaded.bind(
        'recovered',
        const WorkingFile(
          kind: WorkingFileKind.folio,
          path: '/saved.folio',
          fingerprint: 'a',
          revision: 0,
          contentSignature: 'saved',
        ),
      );
      expect(loaded.workspaceDraft, isFalse);
      expect(
        loaded.pending({'id': 'recovered'}, {'folio': 'saved'}, 0),
        isEmpty,
      );
    },
  );
}
