import 'dart:async';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/agent.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/assistant_ui.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'live Codex reads and proposes; only Apply changes the native engine',
    () async {
      await CoreBridge.initialize();
      final core = CoreBridge();
      final before = await core.call({'op': 'new'});
      final dir = await Directory.systemTemp.createTemp('folio-native-live-');
      Map<String, dynamic>? proposal;
      final calls = <String>[];
      final model = WorkspaceModel(core, '${dir.path}/recovery.folio');
      await model.initialize();
      final ui = AssistantUi(model: model, onAction: (_) {}, onOpen: (_, _) {});
      final agent = CodexAgent(
        workingDirectory: dir.path,
        onTool: (name, args) async {
          calls.add(name);
          if (name == 'read_workspace') return core.call({'op': 'context'});
          if (name == 'read_range') {
            return core.call({'op': 'readRange', ...args});
          }
          if (name == 'present_ui') {
            ui.present(args['messages'] as List);
            return {'shown': true};
          }
          proposal = await core.call({'op': 'propose', 'proposal': args});
          return {
            'status': 'proposed',
            'message': 'Waiting for user Apply. Nothing was changed.',
          };
        },
      );
      addTearDown(() async {
        agent.dispose();
        ui.dispose();
        model.dispose();
        await dir.delete(recursive: true);
      });
      await agent.connect();
      expect(
        agent.signedIn,
        isTrue,
        reason: 'This optional test requires existing Codex sign-in.',
      );
      final completed = Completer<void>();
      agent.addListener(() {
        if (!agent.busy && !completed.isCompleted) completed.complete();
      });
      await agent.send(
        'Read the current workspace using read_workspace, then read_range for budget!A3:D9. First show an A2UI surface named budget_preview with a root Column containing a Text and WorkbookRange for budget A3:D9. Use present_ui and the documented catalog. Then submit exactly one propose_changes call: set budget!B4 to 3000 and append the paragraph "Native integration verified." Keep all other content unchanged. Use the live revision. Do not apply it.',
      );
      await completed.future.timeout(const Duration(seconds: 120));
      expect(agent.lastError, isNull);
      expect(
        calls,
        containsAll([
          'read_workspace',
          'read_range',
          'present_ui',
          'propose_changes',
        ]),
      );
      expect(proposal, isNotNull);
      expect(ui.surfaces, contains('budget_preview'));
      expect(
        (await core.call({'op': 'get'}))['workspace'],
        before['workspace'],
      );
      final applied = await core.call({'op': 'apply', 'proposal': proposal});
      expect(applied['calculated']['budget']['B9'], 20600);
      expect(
        applied['workspace']['document']['content'].last['content'][0]['text'],
        'Native integration verified.',
      );
      final undone = await core.call({'op': 'undo'});
      expect(undone['workspace']['document'], before['workspace']['document']);
      expect(undone['calculated']['budget']['B9'], 20000);
    },
    skip: !const bool.fromEnvironment('FOLIO_LIVE_TEST'),
    timeout: const Timeout(Duration(minutes: 3)),
  );
}
