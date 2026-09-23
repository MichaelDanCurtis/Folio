import 'dart:convert';
import 'dart:io';

File latestRecoveryFor(Directory directory, Object? id) {
  final files =
      Directory('${directory.path}/Snapshots')
          .listSync(recursive: true, followLinks: false)
          .whereType<File>()
          .where(
            (f) =>
                f.path.endsWith('.folio') &&
                (jsonDecode(f.readAsStringSync()) as Map)['id'] == id,
          )
          .toList()
        ..sort(
          (a, b) => b.statSync().modified.compareTo(a.statSync().modified),
        );
  return files.first;
}
