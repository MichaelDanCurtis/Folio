import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/goal_seek_dialog.dart';

Map<String, dynamic> response(String status) => {
  'token': 42,
  'status': status,
  'reason': status == 'converged'
      ? 'A matching input value was found.'
      : 'The search stopped.',
  'trials': status == 'running' ? 0 : 4,
  'maxTrials': 200,
  'goal': 100.0,
  'originalInput': '2',
  'input': 31.0,
  'result': 100.0,
  'difference': 0.0,
  'canApply': status != 'running',
};
Future<void> open(
  WidgetTester t,
  ChangeNotifier changes,
  Future<Map<String, dynamic>> Function(Map<String, dynamic>) call,
  Future<void> Function(Map<String, dynamic>) apply, {
  bool Function()? current,
}) async {
  await t.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => GoalSeekDialog(
                sheets: const [
                  {'id': 's', 'name': 'Sheet1'},
                  {'id': 'other', 'name': 'Inputs'},
                ],
                sheetId: 's',
                address: 'B1',
                changes: changes,
                isCurrent: current ?? () => true,
                onCall: call,
                onApply: apply,
              ),
            ),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await t.tap(find.text('Open'));
  await t.pumpAndSettle();
}

Future<void> fill(WidgetTester t) async {
  await t.enterText(find.byKey(const Key('goal-target-value')), '100');
  await t.enterText(find.byKey(const Key('goal-changing-cell')), r'$A$1');
}

void main() {
  testWidgets(
    'Goal Seek validates, previews without applying and retries an Apply failure',
    (t) async {
      final changes = ChangeNotifier();
      addTearDown(changes.dispose);
      final requests = <Map<String, dynamic>>[];
      var applications = 0;
      await open(
        t,
        changes,
        (r) async {
          requests.add(r);
          return response(r['op'] == 'goalSeekStart' ? 'running' : 'converged');
        },
        (r) async {
          expect(r['op'], 'goalSeekAccept');
          expect(r['token'], 42);
          expect(r['acceptBest'], false);
          applications++;
          if (applications == 1) {
            throw StateError('Validation rejected this input');
          }
        },
      );
      await t.tap(find.byKey(const Key('goal-run')));
      await t.pumpAndSettle();
      expect(find.byKey(const Key('goal-error')), findsOneWidget);
      expect(requests, isEmpty);
      await fill(t);
      await t.tap(find.byKey(const Key('goal-run')));
      await t.pumpAndSettle();
      expect(requests.first['changing'], {'sheetId': 's', 'address': 'A1'});
      expect(requests.first['goal'], 100.0);
      expect(applications, 0);
      expect(find.text('Proposed input'), findsOneWidget);
      await t.tap(find.byKey(const Key('goal-apply')));
      await t.pumpAndSettle();
      expect(find.textContaining('Validation rejected'), findsOneWidget);
      await t.tap(find.byKey(const Key('goal-apply')));
      await t.pumpAndSettle();
      expect(applications, 2);
      expect(find.byType(GoalSeekDialog), findsNothing);
      expect(requests.last['op'], 'goalSeekCancel');
    },
  );

  testWidgets(
    'Stop retains the best trial but requires the explicit best-value action',
    (t) async {
      final changes = ChangeNotifier();
      addTearDown(changes.dispose);
      final step = Completer<Map<String, dynamic>>();
      final requests = <Map<String, dynamic>>[];
      Map<String, dynamic>? applied;
      await open(t, changes, (r) async {
        requests.add(r);
        if (r['op'] == 'goalSeekStep') return step.future;
        return response(r['op'] == 'goalSeekStart' ? 'running' : 'stopped');
      }, (r) async => applied = r);
      await fill(t);
      await t.tap(find.byKey(const Key('goal-run')));
      await t.pump();
      await t.pump();
      await t.tap(find.byKey(const Key('goal-stop')));
      await t.pump();
      step.complete(response('running'));
      await t.pumpAndSettle();
      expect(requests.any((r) => r['op'] == 'goalSeekStop'), isTrue);
      expect(find.text('Apply best value'), findsOneWidget);
      await t.tap(find.text('Apply best value'));
      await t.pumpAndSettle();
      expect(applied?['acceptBest'], isTrue);
    },
  );

  testWidgets(
    'Closing during Start cancels a late token without applying anything',
    (t) async {
      final changes = ChangeNotifier();
      addTearDown(changes.dispose);
      final started = Completer<Map<String, dynamic>>();
      final requests = <Map<String, dynamic>>[];
      await open(t, changes, (r) async {
        requests.add(r);
        return r['op'] == 'goalSeekStart'
            ? started.future
            : response('stopped');
      }, (r) async => fail('Must not apply'));
      await fill(t);
      await t.tap(find.byKey(const Key('goal-run')));
      await t.pump();
      await t.tap(find.byKey(const Key('goal-cancel')));
      await t.pumpAndSettle();
      started.complete(response('running'));
      await t.pumpAndSettle();
      expect(requests.last, {'op': 'goalSeekCancel', 'token': 42});
      expect(find.byType(GoalSeekDialog), findsNothing);
      expect(t.takeException(), isNull);
    },
  );

  testWidgets(
    'A changed workbook rejects late trials and editing clears a completed result',
    (t) async {
      final changes = ChangeNotifier();
      addTearDown(changes.dispose);
      var current = true;
      final step = Completer<Map<String, dynamic>>();
      final requests = <Map<String, dynamic>>[];
      await open(
        t,
        changes,
        (r) async {
          requests.add(r);
          return r['op'] == 'goalSeekStep' ? step.future : response('running');
        },
        (r) async => fail('Must not apply'),
        current: () => current,
      );
      await fill(t);
      await t.tap(find.byKey(const Key('goal-run')));
      await t.pump();
      await t.pump();
      current = false;
      changes.notifyListeners();
      await t.pump();
      step.complete(response('converged'));
      await t.pumpAndSettle();
      expect(find.textContaining('workbook changed'), findsOneWidget);
      expect(find.byKey(const Key('goal-apply')), findsNothing);
      expect(requests.last['op'], 'goalSeekCancel');
      await t.tap(find.byKey(const Key('goal-cancel')));
      await t.pumpAndSettle();
      current = true;
      await open(
        t,
        changes,
        (r) async =>
            response(r['op'] == 'goalSeekStart' ? 'running' : 'converged'),
        (r) async => fail('Must not apply'),
      );
      await fill(t);
      await t.tap(find.byKey(const Key('goal-run')));
      await t.pumpAndSettle();
      expect(find.byKey(const Key('goal-apply')), findsOneWidget);
      await t.enterText(find.byKey(const Key('goal-target-value')), '50%');
      await t.pumpAndSettle();
      expect(find.byKey(const Key('goal-apply')), findsNothing);
      await t.tap(find.byKey(const Key('goal-cancel')));
      await t.pumpAndSettle();
    },
  );
}
