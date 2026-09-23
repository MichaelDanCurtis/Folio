import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/agent.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/proofreading.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'live AI grammar returns small reviewable corrections and preserves facts',
    () async {
      await CoreBridge.initialize();
      final core = CoreBridge();
      await core.call({'op': 'new'});
      final editor = q.QuillController(
        document: q.Document()
          ..insert(
            0,
            'We recieved an letter yesterday.\nThe total is \$25,000.\nKeep this paragraph unchanged.',
          ),
        selection: const TextSelection.collapsed(offset: 0),
      );
      editor.formatText(3, 8, q.Attribute.bold);
      final before = await core.call({
        'op': 'document',
        'document': deltaToDocument(editor.document.toDelta()),
      });
      final dir = await Directory.systemTemp.createTemp('folio-live-grammar-');
      WritingReviewSnapshot? review;
      Map<String, dynamic>? proposal;
      final agent = CodexAgent(
        workingDirectory: dir.path,
        onTool: (name, args) async {
          if (name == 'read_workspace') {
            final result = await core.call({'op': 'context'});
            review = WritingReviewSnapshot.capture(
              editor,
              before['workspace']['revision'],
              before['workspace']['id'],
            );
            result['proofreading'] = review!.toJson();
            return result;
          }
          if (name != 'propose_changes' ||
              args['proofreadEdits'] is! List ||
              review == null) {
            throw StateError(
              'Read workspace and use proofreadEdits for this grammar review.',
            );
          }
          final edits = review!.resolve(
            editor,
            before['workspace']['revision'],
            before['workspace']['id'],
            args['proofreadEdits'],
          );
          args['document'] = applyWritingEdits(editor, edits);
          proposal = await core.call({'op': 'propose', 'proposal': args});
          return {
            'status': 'proposed',
            'message': 'Waiting for user Apply. Nothing has changed.',
          };
        },
      );
      addTearDown(() async {
        agent.dispose();
        editor.dispose();
        await dir.delete(recursive: true);
      });
      await agent.connect();
      expect(agent.signedIn, isTrue);
      final done = Completer<void>();
      agent.addListener(() {
        if (!agent.busy && !done.isCompleted) done.complete();
      });
      await agent.send(
        'Check spelling and grammar in the document using English US. Read read_workspace.proofreading. '
        'Correct the misspelling and the incorrect article in the first sentence using small proofreadEdits, '
        'then submit exactly one propose_changes for review. Keep facts, figures, voice and the other paragraphs unchanged. '
        'Do not replace whole paragraphs or use replaceDocument. Nothing is applied yet.',
      );
      await done.future.timeout(const Duration(seconds: 120));
      expect(agent.lastError, isNull);
      expect(proposal, isNotNull);
      expect(
        (await core.call({'op': 'get'}))['workspace'],
        before['workspace'],
      );
      final applied = await core.call({'op': 'apply', 'proposal': proposal});
      final content = nodeText(applied['workspace']['document']);
      expect(content, contains('We received a letter yesterday.'));
      expect(content, contains('The total is \$25,000.'));
      expect(content, contains('Keep this paragraph unchanged.'));
      final corrected = documentToDelta(
        applied['workspace']['document'],
      ).toJson();
      expect(
        corrected.any(
          (op) =>
              op['insert'] == 'received' && op['attributes']?['bold'] == true,
        ),
        isTrue,
      );
      final undone = await core.call({'op': 'undo'});
      expect(undone['workspace']['document'], before['workspace']['document']);
    },
    skip: !const bool.fromEnvironment('FOLIO_LIVE_TEST'),
    timeout: const Timeout(Duration(minutes: 3)),
  );
}
