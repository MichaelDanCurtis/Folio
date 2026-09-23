import 'draft_text_field.dart';
import 'package:flutter/material.dart';
import 'proofreading.dart';

class WritingPanel extends StatelessWidget {
  final ProofreadingController controller;
  final VoidCallback onClose;
  final void Function(WritingIssue) onSelect;
  final void Function(WritingIssue, String) onFix;
  final void Function(bool) onAi;
  final void Function(Future<void> Function()) guard;
  final bool aiBusy;
  const WritingPanel({
    super.key,
    required this.controller,
    required this.onClose,
    required this.onSelect,
    required this.onFix,
    required this.onAi,
    required this.guard,
    required this.aiBusy,
  });

  Future<void> dictionary(BuildContext context) async {
    final input = TextEditingController();
    await showDialog<void>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, update) => AlertDialog(
          title: const Text('Personal dictionary'),
          content: SizedBox(
            width: 400,
            height: 360,
            child: Column(
              children: [
                const Text(
                  'Names and specialist terms saved on this device. These words will not be flagged for spelling.',
                ),
                Row(
                  children: [
                    Expanded(
                      child: draftTextField(
                        controller: input,
                        decoration: const InputDecoration(
                          labelText: 'Add a word',
                        ),
                      ),
                    ),
                    IconButton(
                      tooltip: 'Add word',
                      onPressed: () => guard(() async {
                        final value = input.text.trim();
                        if (value.isEmpty) return;
                        await controller.learn(value);
                        input.clear();
                        update(() {});
                      }),
                      icon: const Icon(Icons.add),
                    ),
                  ],
                ),
                Expanded(
                  child: ListView(
                    children: [
                      for (final word in controller.words.toList()..sort())
                        ListTile(
                          dense: true,
                          title: Text(word),
                          trailing: IconButton(
                            tooltip: 'Remove $word',
                            icon: const Icon(Icons.close, size: 16),
                            onPressed: () => guard(() async {
                              await controller.forget(word);
                              update(() {});
                            }),
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Done'),
            ),
          ],
        ),
      ),
    );
    input.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: controller,
    builder: (context, _) => Container(
      decoration: const BoxDecoration(
        color: Colors.white,
        border: Border(left: BorderSide(color: Color(0xffdfe3e8))),
      ),
      child: Material(
        color: Colors.transparent,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(18, 14, 8, 8),
              child: Row(
                children: [
                  const Icon(
                    Icons.spellcheck,
                    color: Color(0xff185abd),
                    size: 21,
                  ),
                  const SizedBox(width: 10),
                  const Expanded(
                    child: Text(
                      'Writing Editor',
                      style: TextStyle(
                        fontWeight: FontWeight.w600,
                        fontSize: 16,
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: 'Back to assistant',
                    onPressed: onClose,
                    icon: const Icon(Icons.close, size: 19),
                  ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 18),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text(
                    'Spelling & grammar · On this device',
                    style: TextStyle(fontSize: 11, color: Color(0xff616161)),
                  ),
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String>(
                    initialValue: controller.language,
                    key: ValueKey(controller.language),
                    isExpanded: true,
                    decoration: const InputDecoration(
                      labelText: 'Proofing language',
                      isDense: true,
                      border: OutlineInputBorder(),
                    ),
                    items: [
                      for (final entry in proofreadingLanguages.entries)
                        DropdownMenuItem(
                          value: entry.key,
                          child: Text(
                            entry.value,
                            style: const TextStyle(fontSize: 12),
                          ),
                        ),
                    ],
                    onChanged: (v) =>
                        guard(() => controller.configure(language: v)),
                  ),
                  SwitchListTile.adaptive(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    title: const Text(
                      'Check as I type',
                      style: TextStyle(fontSize: 12),
                    ),
                    value: controller.enabled,
                    onChanged: (v) =>
                        guard(() => controller.configure(enabled: v)),
                  ),
                  Row(
                    children: [
                      FilterChip(
                        label: const Text('Grammar'),
                        selected: controller.grammar,
                        onSelected: (v) =>
                            guard(() => controller.configure(grammar: v)),
                      ),
                      const SizedBox(width: 8),
                      FilterChip(
                        label: const Text('Style'),
                        selected: controller.style,
                        onSelected: (v) =>
                            guard(() => controller.configure(style: v)),
                      ),
                    ],
                  ),
                  Row(
                    children: [
                      TextButton.icon(
                        onPressed: () => guard(() => dictionary(context)),
                        icon: const Icon(Icons.menu_book, size: 16),
                        label: const Text('Dictionary'),
                      ),
                      const Spacer(),
                      IconButton(
                        tooltip: 'Check again',
                        onPressed: controller.enabled
                            ? () => guard(controller.checkNow)
                            : null,
                        icon: const Icon(Icons.refresh, size: 18),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.all(18),
                children: [
                  if (controller.settingsError != null)
                    Text(
                      controller.settingsError!,
                      style: const TextStyle(color: Colors.deepOrange),
                    ),
                  if (controller.error != null)
                    Text(
                      controller.error!,
                      style: const TextStyle(color: Colors.deepOrange),
                    ),
                  if (!controller.enabled)
                    const Text(
                      'Automatic checks are off. AI review is still available.',
                    )
                  else if (controller.busy)
                    const Text('Checking your writing…')
                  else if (controller.error == null)
                    Text(
                      controller.issues.isEmpty
                          ? 'No suggestions in the checked text.'
                          : '${controller.issues.length} suggestions',
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                  if (controller.partial)
                    const Padding(
                      padding: EdgeInsets.only(top: 8),
                      child: Text(
                        'Partial check: passages over 12,000 characters and text beyond the first 100,000 characters are skipped. Shorten long passages to check them.',
                        style: TextStyle(
                          fontSize: 11,
                          color: Colors.deepOrange,
                        ),
                      ),
                    ),
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 8),
                    child: Text(
                      'Checks cover document text. Tables, linked objects and code are excluded.',
                      style: TextStyle(fontSize: 10, color: Color(0xff616161)),
                    ),
                  ),
                  for (final issue in controller.issues)
                    Container(
                      key: ValueKey(issue.key),
                      margin: const EdgeInsets.only(bottom: 12),
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        border: Border.all(color: const Color(0xffe2e5ea)),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          Text(
                            issue.category.toUpperCase(),
                            style: TextStyle(
                              color: issue.color,
                              fontSize: 10,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          TextButton(
                            onPressed: () => onSelect(issue),
                            style: TextButton.styleFrom(
                              alignment: Alignment.centerLeft,
                              padding: EdgeInsets.zero,
                            ),
                            child: Text(
                              issue.text,
                              style: const TextStyle(
                                fontSize: 14,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                          Text(
                            issue.message,
                            style: const TextStyle(fontSize: 12, height: 1.4),
                          ),
                          const SizedBox(height: 8),
                          Wrap(
                            spacing: 6,
                            runSpacing: 6,
                            children: [
                              for (final replacement in issue.suggestions)
                                ActionChip(
                                  key: ValueKey(
                                    'fix-${issue.start}-$replacement',
                                  ),
                                  label: Text(
                                    replacement.isEmpty
                                        ? 'Remove'
                                        : replacement,
                                    style: const TextStyle(fontSize: 12),
                                  ),
                                  onPressed: () => onFix(issue, replacement),
                                ),
                            ],
                          ),
                          Wrap(
                            spacing: 4,
                            children: [
                              TextButton(
                                onPressed: () => controller.ignore(issue),
                                child: const Text(
                                  'Ignore',
                                  style: TextStyle(fontSize: 11),
                                ),
                              ),
                              if (issue.category == 'spelling')
                                TextButton(
                                  onPressed: () =>
                                      guard(() => controller.learn(issue.text)),
                                  child: const Text(
                                    'Add to dictionary',
                                    style: TextStyle(fontSize: 11),
                                  ),
                                ),
                            ],
                          ),
                        ],
                      ),
                    ),
                ],
              ),
            ),
            Container(
              padding: const EdgeInsets.all(18),
              color: const Color(0xfff3f6fc),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text(
                    'A second look with AI',
                    style: TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 6),
                  const Text(
                    'Review grammar in context while keeping your voice. Suggested changes wait for your approval.',
                    style: TextStyle(fontSize: 11, height: 1.4),
                  ),
                  const SizedBox(height: 10),
                  FilledButton.icon(
                    key: const Key('ai-grammar-selection'),
                    onPressed: aiBusy ? null : () => onAi(false),
                    icon: const Icon(Icons.auto_awesome, size: 16),
                    label: const Text('Check selection / paragraph'),
                  ),
                  TextButton(
                    key: const Key('ai-grammar-document'),
                    onPressed: aiBusy ? null : () => onAi(true),
                    child: const Text('Check entire document'),
                  ),
                  const Text(
                    'AI review uses Codex and sends context to OpenAI. Built-in checks stay on this device.',
                    style: TextStyle(
                      fontSize: 10,
                      color: Color(0xff616161),
                      height: 1.4,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    ),
  );
}
