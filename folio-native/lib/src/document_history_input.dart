import 'dart:async';
import 'dart:collection';
import 'dart:math' as math;
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'document_edit_tracking.dart';

sealed class DocumentInputCommand {}

class DocumentHistoryCommand extends DocumentInputCommand {
  final bool redo;
  final done = Completer<void>();
  DocumentHistoryCommand(this.redo);
}

class QueuedDocumentEdit extends DocumentInputCommand {
  Delta change;
  LogicalTextEdits? textEdits;
  QueuedDocumentEdit(this.change, this.textEdits);
}

/// Input remains visible while workspace history crosses the asynchronous engine
/// boundary. Keep intent order, and move queued edits through the committed
/// history change instead of submitting snapshots of the superseded document.
class DocumentHistoryInput {
  Delta base;
  final commands = Queue<DocumentInputCommand>();
  DocumentHistoryInput(this.base);

  Delta get visible {
    var value = base;
    for (final command in commands) {
      if (command is QueuedDocumentEdit) value = value.compose(command.change);
    }
    return value;
  }

  /// Returns the exact change to the optimistic view (including later typing),
  /// which also maps its selection without diffing repeated local text.
  Delta rebase(Delta committed, {Delta? change}) {
    var remote = change ?? base.diff(committed);
    var old = base;
    for (final command in commands) {
      if (command is! QueuedDocumentEdit) continue;
      final local = command.change;
      command.textEdits = rebaseDocumentTextEdits(
        old,
        remote,
        command.textEdits,
      );
      command.change = _transform(
        remote,
        local,
        insertPriority: true,
        attributePriority: false,
      );
      remote = _transform(
        local,
        remote,
        insertPriority: false,
        attributePriority: true,
      );
      old = old.compose(local);
    }
    base = committed;
    return remote;
  }
}

// The earlier history operation inserts before later input at a shared offset;
// later explicit formatting wins when both operations touch the same attribute.
Delta _transform(
  Delta first,
  Delta second, {
  required bool insertPriority,
  required bool attributePriority,
}) {
  final a = DeltaIterator(first), b = DeltaIterator(second);
  final result = Delta();
  while (a.hasNext || b.hasNext) {
    if (a.isNextInsert && (insertPriority || !b.isNextInsert)) {
      result.retain(a.next().length!);
    } else if (b.isNextInsert) {
      result.push(b.next());
    } else {
      final length = math.min(a.peekLength(), b.peekLength());
      final left = a.next(length), right = b.next(length);
      if (left.isDelete) continue;
      if (right.isDelete) {
        result.delete(length);
      } else {
        result.retain(
          length,
          Delta.transformAttributes(
            left.attributes,
            right.attributes,
            attributePriority,
          ),
        );
      }
    }
  }
  return result..trim();
}
