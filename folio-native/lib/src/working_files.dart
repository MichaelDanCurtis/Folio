import 'dart:convert';
import 'dart:io';
import 'package:crypto/crypto.dart';
import 'package:flutter/services.dart';

const fileAccessChannel = MethodChannel('folio/file-access');

enum WorkingFileKind { folio, docx, xlsx }

WorkingFileKind workingFileKindForPath(String path) =>
    switch (path.split('.').last.toLowerCase()) {
      'folio' => WorkingFileKind.folio,
      'docx' || 'dotx' => WorkingFileKind.docx,
      'xlsx' || 'xltx' => WorkingFileKind.xlsx,
      _ => throw ArgumentError('Choose a Folio, Word or Excel file.'),
    };

enum LeaveFilesDecision { proceed, discard, cancel }

class PendingWorkingFile {
  const PendingWorkingFile(this.kind, [this.file]);
  final WorkingFileKind kind;
  final WorkingFile? file;
  String get label =>
      file?.path ??
      (kind == WorkingFileKind.folio
          ? 'Recovered workspace (not saved to a file)'
          : kind == WorkingFileKind.docx
          ? 'This document (not saved to a file)'
          : 'This workbook (not saved to a file)');
}

class WorkingFile {
  const WorkingFile({
    required this.kind,
    required this.path,
    required this.fingerprint,
    required this.revision,
    this.bookmark,
    this.sourceKey,
    this.contentSignature,
  });
  final WorkingFileKind kind;
  final String path, fingerprint;
  final int revision;
  final String? bookmark, sourceKey, contentSignature;
  String get name => File(path).uri.pathSegments.last;
  String get format => switch (path.split('.').last.toLowerCase()) {
    'dotx' when kind == WorkingFileKind.docx => 'dotx',
    'xltx' when kind == WorkingFileKind.xlsx => 'xltx',
    _ => kind.name,
  };
  Map<String, dynamic> toJson() => {
    'kind': kind.name,
    'path': path,
    'fingerprint': fingerprint,
    'revision': revision,
    'bookmark': bookmark,
    'sourceKey': sourceKey,
    'contentSignature': contentSignature,
  };
  factory WorkingFile.fromJson(Map value) => WorkingFile(
    kind: WorkingFileKind.values.byName(value['kind'] as String),
    path: value['path'] as String,
    fingerprint: value['fingerprint'] as String,
    revision: value['revision'] as int,
    bookmark: value['bookmark'] as String?,
    sourceKey: value['sourceKey'] as String?,
    contentSignature: value['contentSignature'] as String?,
  );

  bool isDirty(Map states, int currentRevision) => contentSignature == null
      ? revision != currentRevision
      : contentSignature != states[kind.name];
}

class FileAccessLease {
  FileAccessLease(this.path, this.bookmark, this.close);
  final String path;
  final String? bookmark;
  final Future<void> Function() close;
}

class WorkingFileAccess {
  Future<String?> capture(String path) async => Platform.isMacOS
      ? fileAccessChannel.invokeMethod<String>('capture', {'path': path})
      : null;

  Future<FileAccessLease> acquire(WorkingFile file) async {
    if (Platform.isMacOS && file.bookmark == null) {
      throw StateError('Choose Save As to grant access to this file again.');
    }
    return acquireLocation(file.path, file.bookmark);
  }

  Future<FileAccessLease> acquireLocation(String path, String? bookmark) async {
    if (!Platform.isMacOS) return FileAccessLease(path, null, () async {});
    if (bookmark == null) {
      throw StateError('Locate this file to grant access again.');
    }
    final value = await fileAccessChannel.invokeMapMethod<String, dynamic>(
      'acquire',
      {'bookmark': bookmark},
    );
    return FileAccessLease(
      value!['path'] as String,
      value['bookmark'] as String,
      () => fileAccessChannel.invokeMethod<void>('release', {
        'lease': value['lease'],
      }),
    );
  }
}

Future<String> fileFingerprint(String path) async =>
    (await sha256.bind(File(path).openRead()).first).toString();

/// Local session metadata only. File grants never travel inside a .folio file
/// and are never taken from imported Office content or assistant proposals.
class WorkingFiles {
  WorkingFiles(this.storePath, {WorkingFileAccess? access})
    : access = access ?? WorkingFileAccess();
  final String storePath;
  final WorkingFileAccess access;
  String? workspaceId;
  Map<WorkingFileKind, WorkingFile> _files = {};
  Map<String, dynamic> _untitled = {};
  bool workspaceDraft = false;

  Future<void> load(
    String id, {
    Map? workspace,
    Map? fileStates,
    bool pristine = false,
  }) async {
    workspaceId = id;
    _files = {};
    _untitled = {};
    workspaceDraft = false;
    final file = File(storePath);
    if (await file.exists()) {
      final stored = jsonDecode(await file.readAsString()) as Map;
      if (stored['workspaceId'] == id && stored['pending'] == true) return;
      if (stored['workspaceId'] == id) {
        _files = {
          for (final value in stored['files'] as List)
            WorkingFileKind.values.byName(value['kind'] as String):
                WorkingFile.fromJson(value as Map),
        };
        _untitled = Map<String, dynamic>.from(stored['untitled'] as Map? ?? {});
        workspaceDraft = stored['workspaceDraft'] == true;
        return;
      }
    }
    if (pristine && workspace != null && fileStates != null) {
      _untitled = _initialStates(workspace, fileStates);
      await _persist(id, _files);
    }
  }

  static Map<String, dynamic> _initialStates(Map workspace, Map states) => {
    for (final kind in [WorkingFileKind.docx, WorkingFileKind.xlsx])
      kind.name: {
        'sourceKey': sourceKey(workspace, kind),
        'signature': states[kind.name],
      },
  };

  bool untitledDirty(Map workspace, Map states, WorkingFileKind kind) {
    final baseline = _untitled[kind.name] as Map?;
    return workspace['id'] != workspaceId ||
        baseline == null ||
        baseline['sourceKey'] != sourceKey(workspace, kind) ||
        baseline['signature'] != states[kind.name];
  }

  List<PendingWorkingFile> pending(
    Map workspace,
    Map states,
    int revision, {
    WorkingFileKind? replacing,
  }) {
    final result = <PendingWorkingFile>[];
    final whole = bound(workspace, WorkingFileKind.folio);
    if (workspaceDraft && whole == null) {
      return [const PendingWorkingFile(WorkingFileKind.folio)];
    }
    if (whole != null && whole.isDirty(states, revision)) {
      result.add(PendingWorkingFile(WorkingFileKind.folio, whole));
    }
    for (final kind in [WorkingFileKind.docx, WorkingFileKind.xlsx]) {
      if (replacing != null &&
          replacing != WorkingFileKind.folio &&
          replacing != kind) {
        continue;
      }
      final file = bound(workspace, kind);
      if (file != null) {
        if (file.isDirty(states, revision)) {
          result.add(PendingWorkingFile(kind, file));
        }
      } else if (whole == null && untitledDirty(workspace, states, kind)) {
        result.add(PendingWorkingFile(kind));
      }
    }
    return result;
  }

  static String? sourceKey(Map workspace, WorkingFileKind kind) {
    if (kind == WorkingFileKind.folio) return null;
    final meta =
        workspace[kind == WorkingFileKind.docx
                ? 'officeDocument'
                : 'officeWorkbook']
            as Map?;
    return meta?['sourceId'] as String? ??
        meta?['packageId'] as String? ??
        '${workspace['id']}:${kind.name}';
  }

  WorkingFile? bound(Map workspace, WorkingFileKind kind) {
    if (workspace['id'] != workspaceId) return null;
    final file = _files[kind];
    if (file == null) return null;
    final source = sourceKey(workspace, kind);
    // Before source IDs were introduced, native documents used null. Permit
    // that legacy association only for the same workspace's native source.
    return file.sourceKey == source ||
            (file.sourceKey == null &&
                source == '${workspace['id']}:${kind.name}')
        ? file
        : null;
  }

  WorkingFile? active(Map workspace, bool spreadsheet) {
    if (workspace['id'] != workspaceId) return null;
    final kind = spreadsheet ? WorkingFileKind.xlsx : WorkingFileKind.docx;
    return bound(workspace, kind) ?? bound(workspace, WorkingFileKind.folio);
  }

  Future<void> bind(
    String id,
    WorkingFile file, {
    bool replaceAll = false,
  }) async {
    final next = replaceAll || id != workspaceId
        ? <WorkingFileKind, WorkingFile>{}
        : Map.of(_files);
    next[file.kind] = file;
    workspaceId = id;
    _files = next;
    if (file.kind == WorkingFileKind.folio) workspaceDraft = false;
    await _persist(id, next);
  }

  Future<void> reset(
    String id, {
    Map? workspace,
    Map? fileStates,
    bool asDraft = false,
  }) async {
    final untitled = workspace == null || fileStates == null
        ? <String, dynamic>{}
        : _initialStates(workspace, fileStates);
    await _persist(id, {}, untitled: untitled, draft: asDraft);
    workspaceId = id;
    _files = {};
    _untitled = untitled;
    workspaceDraft = asDraft;
  }

  // Reserve the association transition before touching an external destination.
  // If final metadata persistence fails, restart ignores old locations instead
  // of silently routing a later Save back to the pre-Save-As original.
  Future<void> prepareChange() => _persist(workspaceId!, _files, pending: true);
  Future<void> cancelChange() => _persist(workspaceId!, _files);

  Future<void> _persist(
    String id,
    Map<WorkingFileKind, WorkingFile> files, {
    bool pending = false,
    Map<String, dynamic>? untitled,
    bool? draft,
  }) async {
    final target = File(storePath);
    await target.parent.create(recursive: true);
    final temp = File('$storePath.tmp');
    await temp.writeAsString(
      jsonEncode({
        'workspaceId': id,
        'pending': pending,
        'files': files.values.map((f) => f.toJson()).toList(),
        'untitled': untitled ?? _untitled,
        'workspaceDraft': draft ?? workspaceDraft,
      }),
      flush: true,
    );
    await temp.rename(storePath);
  }
}
