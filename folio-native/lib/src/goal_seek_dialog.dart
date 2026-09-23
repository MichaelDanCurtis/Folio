import 'dart:async';
import 'package:flutter/material.dart';
import 'core.dart' show CoreCall;
import 'draft_text_field.dart';

class GoalSeekDialog extends StatefulWidget {
  final List sheets;
  final String sheetId, address;
  final Listenable changes;
  final bool Function() isCurrent;
  final CoreCall onCall;
  final Future<void> Function(Map<String, dynamic>) onApply;
  const GoalSeekDialog({
    super.key,
    required this.sheets,
    required this.sheetId,
    required this.address,
    required this.changes,
    required this.isCurrent,
    required this.onCall,
    required this.onApply,
  });
  @override
  State<GoalSeekDialog> createState() => _GoalSeekDialogState();
}

class _GoalSeekDialogState extends State<GoalSeekDialog> {
  late String targetSheet = widget.sheetId, changingSheet = widget.sheetId;
  late final target = TextEditingController(text: widget.address);
  final changing = TextEditingController();
  final goal = TextEditingController();
  final tolerance = TextEditingController(text: '0.000001');
  final trials = TextEditingController(text: '200');
  Map<String, dynamic>? result;
  String? error;
  bool busy = false, applying = false, stopping = false, stale = false;
  int? token;
  bool get enabled => !busy && !applying && !stale;

  @override
  void initState() {
    super.initState();
    widget.changes.addListener(changed);
  }

  void changed() {
    if (!mounted || applying || widget.isCurrent()) return;
    setState(() {
      stale = true;
      stopping = true;
      result = null;
      error = 'The workbook changed. Close and reopen Goal Seek.';
    });
    unawaited(cancel(token));
  }

  Future<void> cancel(int? value) async {
    if (value == null) return;
    try {
      await widget.onCall({'op': 'goalSeekCancel', 'token': value});
    } catch (_) {
      // The token is isolated and cannot be applied after the dialog closes.
    }
  }

  @override
  void dispose() {
    stopping = true;
    widget.changes.removeListener(changed);
    unawaited(cancel(token));
    for (final c in [target, changing, goal, tolerance, trials]) {
      c.dispose();
    }
    super.dispose();
  }

  void edited(String _) {
    if (result == null && error == null) return;
    unawaited(cancel(token));
    token = null;
    setState(() {
      result = null;
      error = null;
    });
  }

  String cell(String value) => value.trim().replaceAll(r'$', '').toUpperCase();
  double? number(String value) {
    final text = value.trim();
    final percent = text.endsWith('%');
    final parsed = double.tryParse(
      percent ? text.substring(0, text.length - 1) : text,
    );
    return parsed == null ? null : parsed / (percent ? 100 : 1);
  }

  Future<void> run() async {
    if (!enabled) return;
    final wanted = number(goal.text),
        difference = double.tryParse(tolerance.text);
    final maximum = int.tryParse(trials.text);
    final address = RegExp(r'^[A-Z]{1,3}[1-9][0-9]{0,6}$');
    if (!address.hasMatch(cell(target.text)) ||
        !address.hasMatch(cell(changing.text)) ||
        wanted == null ||
        !wanted.isFinite ||
        difference == null ||
        !difference.isFinite ||
        difference <= 0 ||
        maximum == null ||
        maximum < 4 ||
        maximum > 10000) {
      setState(
        () => error =
            'Enter two cell addresses, a numeric target, a positive maximum difference and 4–10,000 trials.',
      );
      return;
    }
    if (!widget.isCurrent()) {
      changed();
      return;
    }
    final old = token;
    setState(() {
      busy = true;
      stopping = false;
      error = null;
      result = null;
      token = null;
    });
    int? runningToken;
    try {
      await cancel(old);
      if (!mounted || stale) return;
      var next = await widget.onCall({
        'op': 'goalSeekStart',
        'target': {'sheetId': targetSheet, 'address': cell(target.text)},
        'changing': {'sheetId': changingSheet, 'address': cell(changing.text)},
        'goal': wanted,
        'tolerance': difference,
        'maxTrials': maximum,
      });
      runningToken = next['token'] as int;
      if (!mounted || stale) {
        await cancel(runningToken);
        return;
      }
      setState(() {
        token = runningToken;
        result = next;
      });
      while (next['status'] == 'running' && mounted && !stale) {
        next = await widget.onCall({
          'op': stopping ? 'goalSeekStop' : 'goalSeekStep',
          'token': runningToken,
        });
        if (!mounted || stale) break;
        setState(() => result = next);
        // The native call performs one trial. Yield between trials so Stop and
        // workspace invalidation can run even with very fast formula models.
        await Future<void>.delayed(Duration.zero);
      }
      if (!mounted || stale) await cancel(runningToken);
    } catch (e) {
      if (mounted && !stale) {
        setState(() {
          error = e.toString();
          if (result != null) {
            result = {
              ...result!,
              'status': 'error',
              'reason': 'The search could not finish.',
              'canApply': false,
            };
          }
        });
      }
      await cancel(runningToken);
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  Future<void> apply() async {
    if (!enabled || result?['canApply'] != true || token == null) return;
    if (!widget.isCurrent()) {
      changed();
      return;
    }
    setState(() {
      applying = true;
      error = null;
    });
    var completed = false;
    try {
      await widget.onApply({
        'op': 'goalSeekAccept',
        'token': token,
        'acceptBest': result!['status'] != 'converged',
      });
      completed = true;
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) {
        setState(() => applying = false);
        if (!completed) changed();
      }
    }
  }

  Widget sheetField(
    String label,
    String value,
    ValueChanged<String> onChange,
    String key,
  ) => DropdownButtonFormField<String>(
    key: ValueKey(key),
    initialValue: value,
    isExpanded: true,
    decoration: InputDecoration(labelText: label),
    items: [
      for (final s in widget.sheets)
        DropdownMenuItem(
          value: s['id'] as String,
          child: Text(s['name'] as String),
        ),
    ],
    onChanged: enabled
        ? (v) {
            if (v != null) {
              edited('');
              setState(() => onChange(v));
            }
          }
        : null,
  );
  Widget field(
    String label,
    TextEditingController c,
    String key, {
    bool autofocus = false,
  }) => draftTextField(
    key: ValueKey(key),
    controller: c,
    enabled: enabled,
    autofocus: autofocus,
    onChanged: edited,
    onSubmitted: enabled ? (_) => unawaited(run()) : null,
    decoration: InputDecoration(labelText: label),
  );
  String show(dynamic v) {
    if (v == null) return '—';
    if (v is! num) return v.toString();
    final parts = v.toStringAsPrecision(12).split('e');
    if (parts[0].contains('.')) {
      parts[0] = parts[0]
          .replaceFirst(RegExp(r'0+$'), '')
          .replaceFirst(RegExp(r'\.$'), '');
    }
    return parts.join('e');
  }

  Widget pair(String label, dynamic value) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 4),
    child: Row(
      children: [
        Expanded(child: Text(label)),
        Tooltip(
          message: value?.toString() ?? '',
          child: SelectableText(show(value)),
        ),
      ],
    ),
  );

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !applying,
    child: AlertDialog(
      key: const Key('goal-seek-dialog'),
      title: const Text('Goal Seek'),
      content: SizedBox(
        width: 600,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'Find an input value that makes a formula reach your target. Review the result before applying it.',
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: sheetField(
                      'Formula worksheet',
                      targetSheet,
                      (v) => targetSheet = v,
                      'goal-target-sheet',
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: field('Set cell', target, 'goal-target-cell'),
                  ),
                ],
              ),
              field('To value', goal, 'goal-target-value', autofocus: true),
              Row(
                children: [
                  Expanded(
                    child: sheetField(
                      'Input worksheet',
                      changingSheet,
                      (v) => changingSheet = v,
                      'goal-changing-sheet',
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: field(
                      'By changing cell',
                      changing,
                      'goal-changing-cell',
                    ),
                  ),
                ],
              ),
              ExpansionTile(
                title: const Text('Search settings'),
                tilePadding: EdgeInsets.zero,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: field(
                          'Maximum difference',
                          tolerance,
                          'goal-tolerance',
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: field(
                          'Maximum trials',
                          trials,
                          'goal-max-trials',
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              if (busy || applying) const LinearProgressIndicator(),
              if (result != null) ...[
                const Divider(),
                Text(
                  result!['status'] == 'running'
                      ? 'Searching…'
                      : result!['reason'] as String,
                  key: const Key('goal-status'),
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                pair(
                  'Original input',
                  result!['originalInput'] == ''
                      ? '(blank)'
                      : result!['originalInput'],
                ),
                pair('Proposed input', result!['input']),
                pair('Target result', result!['goal']),
                pair('Calculated result', result!['result']),
                pair('Difference', result!['difference']),
                pair(
                  'Trials',
                  '${result!['trials']} / ${result!['maxTrials']}',
                ),
                if (result!['lastTrialError'] != null &&
                    result!['canApply'] != true)
                  Text(result!['lastTrialError'] as String),
              ],
              if (error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    error!,
                    key: const Key('goal-error'),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('goal-cancel'),
          onPressed: applying ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        if (busy)
          TextButton(
            key: const Key('goal-stop'),
            onPressed: stopping ? null : () => setState(() => stopping = true),
            child: Text(stopping ? 'Stopping…' : 'Stop'),
          ),
        if (!busy)
          TextButton(
            key: const Key('goal-run'),
            onPressed: enabled ? run : null,
            child: Text(result == null ? 'Find value' : 'Run again'),
          ),
        if (result?['canApply'] == true)
          FilledButton(
            key: const Key('goal-apply'),
            onPressed: enabled ? apply : null,
            child: Text(
              result!['status'] == 'converged' ? 'Apply' : 'Apply best value',
            ),
          ),
      ],
    ),
  );
}
