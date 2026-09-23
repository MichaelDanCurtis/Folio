import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/working_files.dart';

void main() {
  test(
    'first-save tracking persists, distinguishes parts, and treats unknown recovery as unsaved',
    () async {
      final dir = await Directory.systemTemp.createTemp('folio-untitled-');
      addTearDown(() => dir.delete(recursive: true));
      final workspace = {'id': 'new'};
      final states = {
        'docx': 'empty-doc',
        'xlsx': 'empty-book',
        'folio': 'empty',
      };
      final files = WorkingFiles('${dir.path}/files.json');
      await files.load(
        'new',
        workspace: workspace,
        fileStates: states,
        pristine: true,
      );
      expect(files.pending(workspace, states, 0), isEmpty);
      final edited = {...states, 'docx': 'typed'};
      expect(
        files.pending(workspace, edited, 1).single.kind,
        WorkingFileKind.docx,
      );
      expect(files.pending(workspace, edited, 1).single.file, isNull);
      expect(
        files.pending(workspace, edited, 1, replacing: WorkingFileKind.xlsx),
        isEmpty,
      );
      final restored = WorkingFiles(files.storePath);
      await restored.load('new');
      expect(
        restored.pending(workspace, edited, 1).single.kind,
        WorkingFileKind.docx,
      );
      expect(
        restored.pending(workspace, states, 2),
        isEmpty,
        reason: 'Undo restores untitled baseline',
      );
      await restored.bind(
        'new',
        const WorkingFile(
          kind: WorkingFileKind.docx,
          path: '/chosen/new.docx',
          fingerprint: 'bytes',
          revision: 1,
          sourceKey: 'new:docx',
          contentSignature: 'typed',
        ),
      );
      expect(restored.pending(workspace, edited, 2), isEmpty);
      final both = {...edited, 'xlsx': 'entered'};
      expect(
        restored.pending(workspace, both, 3).single.kind,
        WorkingFileKind.xlsx,
      );
      await restored.prepareChange();
      final interrupted = WorkingFiles(files.storePath);
      await interrupted.load(
        'new',
        workspace: workspace,
        fileStates: both,
        pristine: true,
      );
      expect(interrupted.pending(workspace, both, 3), hasLength(2));
      final unknown = WorkingFiles('${dir.path}/legacy.json');
      await unknown.load('old', workspace: {'id': 'old'}, fileStates: states);
      expect(unknown.pending({'id': 'old'}, states, 0), hasLength(2));
      await unknown.reset('new', workspace: workspace, fileStates: states);
      expect(unknown.pending(workspace, states, 0), isEmpty);
      await unknown.bind(
        'new',
        const WorkingFile(
          kind: WorkingFileKind.folio,
          path: '/chosen/all.folio',
          fingerprint: 'all',
          revision: 3,
          contentSignature: 'empty',
        ),
      );
      final wholeChange = unknown.pending(workspace, {
        ...edited,
        'folio': 'changed',
      }, 3);
      expect(wholeChange, hasLength(1));
      expect(
        wholeChange.single.kind,
        WorkingFileKind.folio,
        reason: 'A full workspace file covers unnamed views',
      );
    },
  );
  test(
    'saved signatures persist and Office bindings follow source lineage',
    () async {
      final dir = await Directory.systemTemp.createTemp('folio-file-state-');
      addTearDown(() => dir.delete(recursive: true));
      final files = WorkingFiles('${dir.path}/bindings.json');
      await files.load('a');
      const document = WorkingFile(
        kind: WorkingFileKind.docx,
        path: '/chosen/A.docx',
        fingerprint: 'bytes',
        revision: 1,
        sourceKey: 'source',
        contentSignature: 'saved-content',
      );
      await files.bind('a', document);
      final workspace = {
        'id': 'a',
        'officeDocument': {'packageId': 'edited-package', 'sourceId': 'source'},
      };
      expect(files.active(workspace, false), document);
      expect(document.isDirty({'docx': 'saved-content'}, 400), isFalse);
      expect(document.isDirty({'docx': 'edited-content'}, 1), isTrue);
      final restored = WorkingFiles(files.storePath);
      await restored.load('a');
      expect(
        restored.active(workspace, false)!.contentSignature,
        'saved-content',
      );
      expect(
        restored.active({
          'id': 'a',
          'officeDocument': {'packageId': 'other', 'sourceId': 'new-import'},
        }, false),
        isNull,
      );
      const legacyNative = WorkingFile(
        kind: WorkingFileKind.xlsx,
        path: '/chosen/A.xlsx',
        fingerprint: 'old',
        revision: 2,
      );
      await files.bind('a', legacyNative);
      expect(files.active({'id': 'a'}, true), legacyNative);
      expect(
        files.active({
          'id': 'a',
          'officeWorkbook': {'packageId': 'edited', 'sourceId': 'a:xlsx'},
        }, true),
        legacyNative,
      );
      expect(
        files.active({
          'id': 'a',
          'officeWorkbook': {'packageId': 'imported'},
        }, true),
        isNull,
      );
    },
  );
  test(
    'local bindings follow workspace and Office source identity, never undo into another file',
    () async {
      final dir = await Directory.systemTemp.createTemp(
        'folio-working-bindings-',
      );
      addTearDown(() => dir.delete(recursive: true));
      final files = WorkingFiles('${dir.path}/bindings.json');
      await files.load('workspace-a');
      const native = WorkingFile(
        kind: WorkingFileKind.folio,
        path: '/chosen/A.folio',
        fingerprint: 'native',
        revision: 1,
      );
      const document = WorkingFile(
        kind: WorkingFileKind.docx,
        path: '/chosen/A.docx',
        fingerprint: 'doc',
        revision: 2,
        sourceKey: 'package-a',
      );
      const book = WorkingFile(
        kind: WorkingFileKind.xlsx,
        path: '/chosen/A.xlsx',
        fingerprint: 'book',
        revision: 3,
        sourceKey: 'package-b',
      );
      await files.bind('workspace-a', native);
      await files.bind('workspace-a', document);
      await files.bind('workspace-a', book);
      final workspace = {
        'id': 'workspace-a',
        'officeDocument': {'packageId': 'package-a'},
        'officeWorkbook': {'packageId': 'package-b'},
      };
      expect(files.active(workspace, false), document);
      expect(files.active(workspace, true), book);
      final afterUndo = {
        ...workspace,
        'officeDocument': {'packageId': 'older-source'},
      };
      expect(
        files.active(afterUndo, false),
        native,
        reason: 'Undo of import must not overwrite the now-unrelated DOCX',
      );
      final loaded = WorkingFiles(files.storePath);
      await loaded.load('workspace-a');
      expect(loaded.active(workspace, false)!.path, document.path);
      expect(loaded.active(afterUndo, false)!.path, native.path);
      expect(loaded.active({...workspace, 'id': 'unrelated'}, false), isNull);
      await files.prepareChange();
      final interrupted = WorkingFiles(files.storePath);
      await interrupted.load('workspace-a');
      expect(
        interrupted.active(workspace, false),
        isNull,
        reason:
            'An interrupted Save As must not restore the former destination',
      );
      await files.cancelChange();
      final cancelled = WorkingFiles(files.storePath);
      await cancelled.load('workspace-a');
      expect(cancelled.active(workspace, false)!.path, document.path);
      await files.bind('workspace-a', native, replaceAll: true);
      expect(
        files.active(workspace, true),
        native,
        reason: 'Explicit Open .folio clears both former Office associations',
      );
      File(files.storePath).writeAsStringSync('{bad json');
      await expectLater(files.load('workspace-a'), throwsFormatException);
      expect(files.active(workspace, false), isNull);
      await files.prepareChange();
      final recovered = WorkingFiles(files.storePath);
      await recovered.load('workspace-a');
      expect(recovered.active(workspace, false), isNull);
      await files.reset('new-workspace');
      expect(files.active({'id': 'new-workspace'}, false), isNull);
      expect(files.active(workspace, false), isNull);
    },
  );
}
