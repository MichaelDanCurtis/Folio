import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'rust/frb_generated.dart';
import 'rust/api.dart' as rust;
import 'sheet_autofit.dart';

typedef CoreCall = Future<Map<String, dynamic>> Function(Map<String, dynamic>);

class CoreBridge {
  Future<void> _tail = Future.value();
  static Future<void> initialize() async {
    final contents = File(Platform.resolvedExecutable).parent.parent.path;
    final candidates = [
      Platform.environment['FOLIO_CORE_PATH'],
      const String.fromEnvironment('FOLIO_CORE_PATH'),
      '$contents/Frameworks/libfolio_core.dylib',
      '${Directory.current.path}/rust/target/release/libfolio_core.dylib',
    ];
    final path = candidates.whereType<String>().firstWhere(
      (p) => File(p).existsSync(),
      orElse: () => throw StateError(
        'The Rust engine was not packaged. Run scripts/build_macos.sh.',
      ),
    );
    await RustLib.init(externalLibrary: ExternalLibrary.open(path));
  }

  Future<Map<String, dynamic>> call(Map<String, dynamic> request) =>
      transaction((dispatch) => dispatch(request));

  /// Keep reads, saves and later edits outside the complete prepare/measure/
  /// commit sequence. A failed measurement never publishes half an edit.
  Future<T> transaction<T>(Future<T> Function(CoreCall dispatch) action) {
    final complete = Completer<T>();
    _tail = _tail.then((_) async {
      try {
        complete.complete(await action(_dispatch));
      } catch (e, st) {
        complete.completeError(e, st);
      }
    });
    return complete.future;
  }

  Future<Map<String, dynamic>> _dispatch(Map<String, dynamic> request) async {
    final result =
        jsonDecode(await rust.dispatch(request: jsonEncode(request))) as Map;
    if (result['ok'] != true) throw StateError(result['error'].toString());
    return Map<String, dynamic>.from(result['value']);
  }
}

class WorkspaceModel extends ChangeNotifier {
  final CoreBridge core;
  final String recoveryPath;
  Map<String, dynamic> snapshot = {};
  String saveStatus = 'Opening…';
  String? error;
  String? _documentSaveError;
  int documentEpoch = 0;
  // An open/new document starts a new selection context. Ordinary committed
  // refreshes (including Undo/Redo) retain the current editor position.
  int documentSession = 0;
  Timer? _saveTimer;
  bool recoveryBlocked = false, recovered = false, _closedRecovery = false;
  bool _disposed = false;
  TextScaler rowTextScaler = TextScaler.noScaling;
  TextDirection rowTextDirection = TextDirection.ltr;
  @visibleForTesting
  Future<Map<String, dynamic>> Function(Map<String, dynamic>)? rowMeasurer;
  void _changed() {
    if (!_disposed) notifyListeners();
  }

  WorkspaceModel(this.core, this.recoveryPath);
  Map<String, dynamic> get workspace =>
      Map<String, dynamic>.from(snapshot['workspace'] ?? {});
  List<dynamic> get sheets => workspace['sheets'] ?? [];
  int get revision => workspace['revision'] ?? 0;
  bool get canUndo => snapshot['canUndo'] == true;
  bool get canRedo => snapshot['canRedo'] == true;
  Future<void> initialize() async {
    snapshot = await core.call({'op': 'get'});
    if (File(recoveryPath).existsSync()) {
      try {
        snapshot = await core.call({'op': 'load', 'path': recoveryPath});
        recovered = true;
      } catch (e) {
        final backup =
            '$recoveryPath.unreadable-${DateTime.now().millisecondsSinceEpoch}';
        try {
          await File(recoveryPath).copy(backup);
          error =
              'The saved workspace could not be opened. Its original contents are preserved at $backup. $e';
        } catch (_) {
          recoveryBlocked = true;
          error =
              'The saved workspace could not be opened. Save a copy of new work before closing. The original file remains at $recoveryPath. $e';
        }
      }
    }
    saveStatus = recoveryBlocked
        ? 'Recovery file needs attention'
        : 'Saved on this Mac';
    _changed();
  }

  Future<void> mutate(
    Map<String, dynamic> request, {
    bool refreshDocument = false,
  }) async {
    const measuredEdits = {
      'cells',
      'enterCell',
      'goalSeekAccept',
      'scenarioShow',
      'scenarioSummary',
      'apply',
      'formatCells',
      'conditionalFormat',
      'formatCellText',
      'fillCells',
      'pasteCells',
      'sheetGeometry',
      'structure',
      'table',
      'sortRange',
      'autoFilter',
    };
    snapshot = await core.transaction((dispatch) async {
      if (!measuredEdits.contains(request['op'])) return dispatch(request);
      final prepared = await dispatch({
        'op': 'prepareLayoutEdit',
        'edit': request,
      });
      final token = prepared['layoutToken'];
      try {
        final sizes =
            await (rowMeasurer?.call(prepared) ??
                measureAutomaticRows(
                  prepared,
                  scaler: rowTextScaler,
                  direction: rowTextDirection,
                ));
        if (_disposed) {
          throw StateError('The workspace closed before the edit completed.');
        }
        return await dispatch({
          'op': 'commitLayoutEdit',
          'token': token,
          'sizes': sizes,
        });
      } catch (_) {
        await dispatch({'op': 'cancelLayoutEdit', 'token': token});
        rethrow;
      }
    });
    if (_disposed) return;
    if (request['op'] == 'document') {
      if (error == _documentSaveError) error = null;
      _documentSaveError = null;
    }
    if (request['op'] == 'new' ||
        request['op'] == 'newFromTemplate' ||
        request['op'] == 'load' ||
        (request['op'] == 'importOffice' &&
            ['docx', 'dotx'].contains(
              (request['path'] as String).split('.').last.toLowerCase(),
            ))) {
      documentSession++;
    }
    if (refreshDocument) documentEpoch++;
    saveStatus = recoveryBlocked
        ? 'Save a copy to keep your changes'
        : 'Saving…';
    _changed();
    _saveTimer?.cancel();
    _saveTimer = Timer(
      const Duration(milliseconds: 350),
      () => flush().catchError((Object e) {
        error = e.toString();
        saveStatus = 'Save failed';
        _changed();
      }),
    );
  }

  /// Publish fresh calculations without a save timer, document replacement or
  /// Undo entry. Queue behind complete edits and ignore a superseded snapshot.
  Future<void> refreshCalculations({bool force = false}) async {
    final id = workspace['id'], epoch = documentEpoch;
    final next = await core.transaction((dispatch) async {
      // A calendar check is a no-op unless the local day or offset changed.
      // Keep NOW fixed within an unchanged calendar until explicitly calculated.
      if (!force) {
        final current = await dispatch({'op': 'get'});
        final now = DateTime.now();
        final utcDay = DateTime.utc(now.year, now.month, now.day);
        final day = utcDay.difference(DateTime.utc(1)).inDays + 1;
        if (current['localCalendarDay'] == day &&
            current['localUtcOffset'] == now.timeZoneOffset.inSeconds) {
          return current;
        }
      }
      final prepared = await dispatch({
        'op': 'prepareLayoutEdit',
        'edit': {'op': force ? 'recalculate' : 'refreshCalendar'},
      });
      final token = prepared['layoutToken'];
      try {
        final sizes =
            await (rowMeasurer?.call(prepared) ??
                measureAutomaticRows(
                  prepared,
                  scaler: rowTextScaler,
                  direction: rowTextDirection,
                ));
        if (_disposed) {
          throw StateError('The workspace closed during calculation.');
        }
        final result = await dispatch({
          'op': 'commitLayoutEdit',
          'token': token,
          'sizes': sizes,
        });
        if (!recoveryBlocked &&
            jsonEncode(result['workspace']) != jsonEncode(workspace)) {
          await dispatch({'op': 'save', 'path': recoveryPath});
        }
        return result;
      } catch (_) {
        await dispatch({'op': 'cancelLayoutEdit', 'token': token});
        rethrow;
      }
    });
    if (_disposed ||
        epoch != documentEpoch ||
        id != workspace['id'] ||
        next['workspace']['id'] != id ||
        (next['workspace']['revision'] as int) < revision ||
        (next['evaluatedCells'] as int) <
            (snapshot['evaluatedCells'] as int? ?? 0)) {
      return;
    }
    if (next['evaluatedCells'] == snapshot['evaluatedCells']) return;
    snapshot = next;
    _changed();
  }

  void recordDocumentSaveFailure(Object failure) {
    _documentSaveError = failure.toString();
    error = _documentSaveError;
    saveStatus = 'Document changes not saved';
    _saveTimer?.cancel();
    _changed();
  }

  /// User explicitly discarded the uncommitted editor draft. The serialized
  /// read drains preceding writes without adding a history entry.
  Future<void> restoreCommittedDocument() async {
    snapshot = await core.transaction((dispatch) => dispatch({'op': 'get'}));
    if (_disposed) return;
    if (error == _documentSaveError) error = null;
    _documentSaveError = null;
    documentEpoch++;
    _changed();
    await flush();
  }

  Future<void> flush() async {
    _saveTimer?.cancel();
    if (_closedRecovery) return;
    if (_documentSaveError != null) throw StateError(_documentSaveError!);
    if (recoveryBlocked) {
      throw StateError(
        'Save a copy before closing: the recovery file cannot be overwritten safely.',
      );
    }
    final saved = await core.transaction((dispatch) async {
      // A flush can already be queued when Don't Save closes persistence.
      // Recheck inside the same engine queue as the blank recovery write.
      if (_closedRecovery) return false;
      await dispatch({'op': 'save', 'path': recoveryPath});
      return true;
    });
    if (!saved) return;
    saveStatus = 'Saved on this Mac';
    _changed();
  }

  /// Keep the live model/history intact until the OS actually closes the app.
  /// A failed atomic blank write leaves both edits and recovery available.
  Future<void> discardRecoveryOnClose() async {
    _saveTimer?.cancel();
    await core.transaction((dispatch) async {
      await dispatch({'op': 'saveEmptyWorkspace', 'path': recoveryPath});
      _closedRecovery = true;
    });
    _saveTimer?.cancel();
  }

  Future<void> open(String path, {bool recover = false}) async {
    await flush();
    snapshot = await core.call({
      'op': recover ? 'recoverWorkspace' : 'load',
      'path': path,
    });
    recoveryBlocked = false;
    documentSession++;
    documentEpoch++;
    await flush();
    _changed();
  }

  Future<void> newWorkspace() async {
    await mutate({'op': 'new', 'blank': true}, refreshDocument: true);
    recoveryBlocked = false;
    await flush();
  }

  Future<void> saveCopy(String path) async {
    await core.call({'op': 'save', 'path': path});
  }

  @override
  void dispose() {
    _disposed = true;
    _saveTimer?.cancel();
    super.dispose();
  }
}
