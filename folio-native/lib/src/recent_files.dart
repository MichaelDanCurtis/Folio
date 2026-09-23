import 'dart:convert';
import 'dart:io';
import 'working_files.dart';

class RecentFile {
  const RecentFile({
    required this.kind,
    required this.path,
    this.bookmark,
    this.lastUsed,
    this.pinned = false,
  });
  final WorkingFileKind kind;
  final String path;
  final String? bookmark;
  final DateTime? lastUsed;
  final bool pinned;
  String get name => File(path).uri.pathSegments.last;
  String get format => switch (path.split('.').last.toLowerCase()) {
    'dotx' when kind == WorkingFileKind.docx => 'dotx',
    'xltx' when kind == WorkingFileKind.xlsx => 'xltx',
    _ => kind.name,
  };
  String get key => File(path).absolute.uri.normalizePath().toString();
  RecentFile pin(bool value) => RecentFile(
    kind: kind,
    path: path,
    bookmark: bookmark,
    lastUsed: lastUsed,
    pinned: value,
  );
  Map<String, dynamic> toJson() => {
    'kind': kind.name,
    'path': path,
    'bookmark': bookmark,
    'lastUsed': lastUsed?.toUtc().toIso8601String(),
    'pinned': pinned,
  };
  factory RecentFile.fromJson(Map value) => RecentFile(
    kind: WorkingFileKind.values.byName(value['kind'] as String),
    path: value['path'] as String,
    bookmark: value['bookmark'] as String?,
    lastUsed: value['lastUsed'] == null
        ? null
        : DateTime.parse(value['lastUsed'] as String),
    pinned: value['pinned'] == true,
  );
}

/// Application-owned metadata. Never import grants from document contents.
class RecentFiles {
  RecentFiles(this.storePath, {this.unpinnedLimit = 50});
  final String storePath;
  final int? unpinnedLimit;
  List<RecentFile> _entries = [];
  bool _loaded = false;
  Future<void> _queue = Future.value();
  List<RecentFile> get entries => List.unmodifiable(_entries);

  Future<void> load({String? migrateFrom}) async {
    final file = File(storePath);
    if (await file.exists()) {
      final data = jsonDecode(await file.readAsString()) as Map;
      _entries = _sorted(
        (data['files'] as List)
            .map((v) => RecentFile.fromJson(v as Map))
            .toList(),
      );
    } else {
      final legacy = migrateFrom == null ? null : File(migrateFrom);
      var entries = <RecentFile>[];
      if (legacy != null && await legacy.exists()) {
        final data = jsonDecode(await legacy.readAsString()) as Map;
        entries = (data['files'] as List? ?? [])
            .map((v) => RecentFile.fromJson(v as Map))
            .toList();
      }
      // Persist even an empty migration so removed history cannot reappear.
      await _write(_sorted(entries));
      _entries = _sorted(entries);
    }
    _loaded = true;
  }

  List<RecentFile> _sorted(List<RecentFile> entries) {
    final unique = <String, RecentFile>{};
    for (final entry in entries) {
      unique.putIfAbsent(entry.key, () => entry);
    }
    final sorted = unique.values.toList()
      ..sort((a, b) {
        if (a.pinned != b.pinned) return a.pinned ? -1 : 1;
        return (b.lastUsed?.millisecondsSinceEpoch ?? 0).compareTo(
          a.lastUsed?.millisecondsSinceEpoch ?? 0,
        );
      });
    var unpinned = 0;
    return sorted
        .where(
          (e) =>
              e.pinned || unpinnedLimit == null || ++unpinned <= unpinnedLimit!,
        )
        .toList();
  }

  Future<void> _update(List<RecentFile> Function() next) {
    final operation = _queue.then((_) async {
      if (!_loaded) {
        throw StateError(
          'Recent files could not be loaded. Existing history has been preserved.',
        );
      }
      final updated = _sorted(next());
      await _write(updated);
      _entries = updated;
    });
    _queue = operation.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return operation;
  }

  Future<void> remember(
    WorkingFileKind kind,
    String path,
    String? bookmark, {
    String? previousPath,
  }) => _update(() {
    final next = RecentFile(
      kind: kind,
      path: path,
      bookmark: bookmark,
      lastUsed: DateTime.now().toUtc(),
    );
    final oldKey = previousPath == null
        ? next.key
        : File(previousPath).absolute.uri.normalizePath().toString();
    final previous = _entries.where(
      (e) => e.key == next.key || e.key == oldKey,
    );
    return [
      next.pin(previous.any((e) => e.pinned)),
      ..._entries.where((e) => e.key != next.key && e.key != oldKey),
    ];
  });
  Future<void> pin(RecentFile file, bool value) => _update(
    () => [for (final e in _entries) e.key == file.key ? e.pin(value) : e],
  );
  Future<void> remove(RecentFile file) =>
      _update(() => _entries.where((e) => e.key != file.key).toList());
  Future<void> clearUnpinned() =>
      _update(() => _entries.where((e) => e.pinned).toList());

  Future<void> _write(List<RecentFile> entries) async {
    final target = File(storePath);
    await target.parent.create(recursive: true);
    final temp = File('$storePath.tmp');
    await temp.writeAsString(
      jsonEncode({
        'version': 1,
        'files': entries.map((e) => e.toJson()).toList(),
      }),
      flush: true,
    );
    await temp.rename(storePath);
  }
}

class RecoveryCopy {
  const RecoveryCopy(this.path, this.modified, {this.description, this.reason});
  final String path;
  final DateTime modified;
  final Map<String, dynamic>? description;
  final String? reason;
  String get title => description?['title'] is String
      ? description!['title'] as String
      : 'Recovery copy';
}

class RecoveryCatalog {
  RecoveryCatalog(this.directory);
  final String directory;

  Future<String> reserve() async {
    final root = Directory(directory);
    await root.create(recursive: true);
    // OS-created unique directories prevent collisions, overwrite, and any
    // user-controlled workspace ID or title from becoming a path component.
    final folder = await root.createTemp(
      'copy-${DateTime.now().toUtc().millisecondsSinceEpoch}-',
    );
    return '${folder.path}/Workspace.folio';
  }

  Future<void> describe(String path, Map saved, String reason) async {
    await File('$path.json').writeAsString(
      jsonEncode({
        'description': saved['description'],
        'fingerprint': saved['fingerprint'],
        'reason': reason,
      }),
      flush: true,
    );
  }

  Future<List<RecoveryCopy>> list() async {
    final root = Directory(directory);
    if (!await root.exists()) return [];
    final candidates = <File>[];
    await for (final entity in root.list(followLinks: false)) {
      if (entity is File && entity.path.endsWith('.folio')) {
        candidates.add(entity);
      }
      if (entity is Directory) {
        final path = '${entity.path}/Workspace.folio';
        if (await FileSystemEntity.type(path, followLinks: false) ==
            FileSystemEntityType.file) {
          candidates.add(File(path));
        }
      }
    }
    final copies = <RecoveryCopy>[];
    for (final file in candidates) {
      try {
        Map<String, dynamic>? info;
        try {
          final meta = File('${file.path}.json');
          if (await meta.length() <= 32000) {
            info = Map<String, dynamic>.from(
              jsonDecode(await meta.readAsString()) as Map,
            );
          }
        } catch (_) {
          /* Legacy or damaged labels do not hide the snapshot. */
        }
        copies.add(
          RecoveryCopy(
            file.path,
            (await file.stat()).modified,
            description: info?['description'] is Map
                ? Map<String, dynamic>.from(info!['description'])
                : null,
            reason: info?['reason'] is String
                ? info!['reason'] as String
                : null,
          ),
        );
      } on FileSystemException {
        /* A concurrently removed file is absent. */
      }
    }
    copies.sort((a, b) => b.modified.compareTo(a.modified));
    return copies;
  }
}
