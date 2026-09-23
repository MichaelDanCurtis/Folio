import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'draft_text_field.dart';

class ToggleTrackingIntent extends Intent {
  const ToggleTrackingIntent();
}

const trackingShortcut = SingleActivator(
  LogicalKeyboardKey.keyE,
  control: true,
  shift: true,
);

class DocumentTrackingDialog extends StatefulWidget {
  final Map settings;
  final String? unavailableReason;
  final Future<void> Function(Map<String, dynamic>) onApply;
  const DocumentTrackingDialog({
    super.key,
    required this.settings,
    required this.onApply,
    this.unavailableReason,
  });
  @override
  State<DocumentTrackingDialog> createState() => _DocumentTrackingDialogState();
}

class _DocumentTrackingDialogState extends State<DocumentTrackingDialog> {
  late final author = TextEditingController(
    text: widget.settings['author'] as String? ?? '',
  );
  late bool enabled = widget.settings['enabled'] == true;
  late bool formatting = widget.settings['formatting'] != false;
  bool busy = false;
  String? error;
  @override
  void dispose() {
    author.dispose();
    super.dispose();
  }

  Future<void> apply() async {
    final name = author.text.trim();
    if (name.isEmpty) {
      setState(() => error = 'Enter the name to show beside your changes.');
      return;
    }
    setState(() {
      busy = true;
      error = null;
    });
    try {
      await widget.onApply({
        'author': name,
        'enabled': enabled,
        'formatting': formatting,
      });
      if (mounted) Navigator.pop(context);
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Recording settings'),
    content: SizedBox(
      width: 430,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Record your edits so they can be accepted or rejected later.',
            ),
            const SizedBox(height: 18),
            draftTextField(
              key: const Key('tracking-author'),
              controller: author,
              autofocus: author.text.isEmpty,
              enabled: !busy,
              maxLength: 120,
              decoration: const InputDecoration(
                labelText: 'Review author',
                helperText: 'This name is saved with each change.',
              ),
            ),
            SwitchListTile.adaptive(
              key: const Key('tracking-enabled'),
              contentPadding: EdgeInsets.zero,
              title: const Text('Record changes'),
              value: enabled,
              onChanged: busy ? null : (v) => setState(() => enabled = v),
            ),
            CheckboxListTile(
              key: const Key('tracking-formatting'),
              contentPadding: EdgeInsets.zero,
              title: const Text('Track formatting changes'),
              value: formatting,
              onChanged: busy
                  ? null
                  : (v) => setState(() => formatting = v ?? true),
            ),
            const Text(
              'Turning recording off keeps earlier changes available for review.',
            ),
            if (widget.unavailableReason != null) ...[
              const SizedBox(height: 12),
              Text(
                widget.unavailableReason!,
                style: const TextStyle(color: Color(0xff9c2929)),
              ),
            ],
            if (error != null) ...[
              const SizedBox(height: 12),
              Text(
                error!,
                key: const Key('tracking-error'),
                style: const TextStyle(color: Color(0xff9c2929)),
              ),
            ],
            if (busy)
              const Padding(
                padding: EdgeInsets.only(top: 12),
                child: LinearProgressIndicator(),
              ),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        key: const Key('tracking-cancel'),
        onPressed: busy ? null : () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('tracking-apply'),
        onPressed: busy || widget.unavailableReason != null ? null : apply,
        child: const Text('Apply'),
      ),
    ],
  );
}
