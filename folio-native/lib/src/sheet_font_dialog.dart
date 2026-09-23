import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'draft_text_field.dart';
import 'sheet_font.dart';

class _CharacterSelectorController extends TextEditingController {
  _CharacterSelectorController(String text) : super(text: text);
  TextSelection highlight = const TextSelection.collapsed(offset: 0);
  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    if (!highlight.isValid ||
        highlight.isCollapsed ||
        highlight.end > text.length) {
      return TextSpan(text: text, style: style);
    }
    return TextSpan(
      style: style,
      children: [
        TextSpan(text: text.substring(0, highlight.start)),
        TextSpan(
          text: text.substring(highlight.start, highlight.end),
          style: TextStyle(
            backgroundColor: Theme.of(
              context,
            ).colorScheme.primary.withValues(alpha: .18),
          ),
        ),
        TextSpan(text: text.substring(highlight.end)),
      ],
    );
  }
}

class SheetFontDialog extends StatefulWidget {
  final List<Map<String, dynamic>> fonts;
  final String sample;
  final Map<String, dynamic>? previewFont;
  final bool allowCharacterSelection;
  const SheetFontDialog({
    required this.fonts,
    required this.sample,
    this.previewFont,
    this.allowCharacterSelection = false,
    super.key,
  });
  @override
  State<SheetFontDialog> createState() => _SheetFontDialogState();
}

class _SheetFontDialogState extends State<SheetFontDialog> {
  static const _channel = MethodChannel('folio/fonts');
  static const underlines = {
    'none': 'None',
    'single': 'Single',
    'double': 'Double',
    'singleAccounting': 'Single accounting',
    'doubleAccounting': 'Double accounting',
  };
  static const verticals = {
    'baseline': 'Normal',
    'superscript': 'Superscript',
    'subscript': 'Subscript',
  };
  static const palette = {
    'automatic': 'Automatic',
    '#242424': 'Black',
    '#FFFFFF': 'White',
    '#808080': 'Gray',
    '#C00000': 'Red',
    '#ED7D31': 'Orange',
    '#FFC000': 'Gold',
    '#107C41': 'Green',
    '#0070C0': 'Blue',
    '#7030A0': 'Purple',
  };
  late Map<String, dynamic> common = commonSheetFont(widget.fonts);
  bool characterMode = false;
  TextSelection characterSelection = const TextSelection.collapsed(offset: 0);
  late final characters = _CharacterSelectorController(widget.sample);
  Map get sourceFont => widget.previewFont ?? widget.fonts.first;
  final changes = <String, dynamic>{};
  late final name = TextEditingController(
    text: common['fontName'] as String? ?? '',
  );
  late final points = TextEditingController(
    text: common['fontSize'] == null
        ? ''
        : fontPoints(common['fontSize'] as num),
  );
  late final color = TextEditingController(
    text: common['color'] as String? ?? '',
  );
  List<String> families = [
    'Aptos',
    'Arial',
    'Calibri',
    'Cambria',
    'Courier New',
    'Georgia',
    'Helvetica',
    'Menlo',
    'Times New Roman',
    'Verdana',
  ];
  bool installedCatalog = false;
  String? error;
  @override
  void initState() {
    super.initState();
    unawaited(loadFonts());
    characters.addListener(selectCharacters);
  }

  void resetControls() {
    changes.clear();
    name.text = common['fontName'] as String? ?? '';
    points.text = common['fontSize'] == null
        ? ''
        : fontPoints(common['fontSize'] as num);
    color.text = common['color'] as String? ?? '';
    error = null;
  }

  void selectCharacters() {
    final selection = characters.selection;
    if (!characterMode ||
        !selection.isValid ||
        selection.isCollapsed ||
        selection == characterSelection) {
      return;
    }
    setState(() {
      characterSelection = selection;
      characters.highlight = selection;
      common = commonSheetFont(
        selectedSheetFontRuns(
          sheetFontRuns(widget.sample, sourceFont),
          selection,
        ).map((r) => r.font),
      );
      resetControls();
    });
  }

  void setCharacterMode(bool enabled) {
    setState(() {
      characterMode = enabled;
      characterSelection = TextSelection(
        baseOffset: 0,
        extentOffset: widget.sample.length,
      );
      characters.highlight = characterSelection;
      characters.selection = characterSelection;
      common = enabled
          ? commonSheetFont(
              sheetFontRuns(widget.sample, sourceFont).map((r) => r.font),
            )
          : commonSheetFont(widget.fonts);
      resetControls();
    });
  }

  Future<void> loadFonts() async {
    try {
      final list = await _channel.invokeListMethod<String>('families');
      if (mounted && list != null && list.isNotEmpty) {
        setState(() {
          families = list;
          installedCatalog = true;
        });
      }
    } on MissingPluginException {
      /* Other hosts retain editable family names. */
    } on PlatformException {
      /* A font name remains usable for portable files. */
    }
  }

  dynamic value(String key) =>
      changes.containsKey(key) ? changes[key] : common[key];
  void setProperty(String key, dynamic v) => setState(() {
    if (v == null || v == common[key]) {
      changes.remove(key);
    } else {
      changes[key] = v;
    }
    error = null;
  });
  void chooseFamily(String family) {
    name.text = family;
    setProperty('fontName', family);
  }

  void chooseColor(String value) {
    color.text = value;
    setProperty('color', value);
  }

  String? validate() {
    final family = changes['fontName'];
    if (family != null &&
        (family.toString().runes.length > 255 ||
            RegExp(r'[\x00-\x1f\x7f-\x9f]').hasMatch(family))) {
      return 'Enter a font name of 1–255 characters.';
    }
    if (changes.containsKey('fontSize')) {
      final n = double.tryParse(points.text.trim());
      if (n == null || !n.isFinite || n < 1 || n > 409 || (n * 2) % 1 != 0) {
        return 'Enter a size from 1 to 409 points, in half-point steps.';
      }
    }
    final c = changes['color'];
    if (c != null &&
        c != 'automatic' &&
        !RegExp(r'^#[0-9a-fA-F]{6}$').hasMatch(c)) {
      return 'Choose a color or enter # followed by six hexadecimal digits.';
    }
    return null;
  }

  void apply() {
    final invalid = validate();
    if (invalid != null) {
      setState(() => error = invalid);
      return;
    }
    final result = Map<String, dynamic>.from(changes);
    if (result.containsKey('fontSize')) {
      result['fontSize'] = double.parse(points.text.trim());
    }
    if (characterMode) {
      result['_textRange'] = {
        'start': characterSelection.start,
        'end': characterSelection.end,
      };
    }
    Navigator.pop(context, result);
  }

  Widget toggle(String key, String label) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      Checkbox(
        key: Key('font-$key'),
        value: value(key) as bool?,
        tristate: true,
        onChanged: (v) => setProperty(key, v),
      ),
      Text(label),
    ],
  );
  Widget choice(String key, String label, Map<String, String> items) {
    final current = value(key) as String?;
    return DropdownButtonFormField<String>(
      key: ValueKey('font-$key-$current'),
      initialValue: current,
      isExpanded: true,
      decoration: InputDecoration(
        labelText: label,
        hintText: 'Mixed — unchanged',
      ),
      items: [
        for (final item in items.entries)
          DropdownMenuItem(value: item.key, child: Text(item.value)),
        if (current != null && !items.containsKey(current))
          DropdownMenuItem(value: current, child: Text('Imported: $current')),
      ],
      onChanged: (v) => setProperty(key, v),
    );
  }

  @override
  Widget build(BuildContext context) {
    final query = name.text.toLowerCase();
    final filtered = families
        .where((f) => f.toLowerCase().contains(query))
        .toList();
    final missing =
        installedCatalog &&
        name.text.isNotEmpty &&
        !families.any((f) => f.toLowerCase() == query);
    final source = sourceFont;
    var preview = <String, dynamic>{
      ...source,
      ...changes,
      '_fontOverrides': {
        ...(source['_fontOverrides'] as Map? ?? {}),
        ...changes,
      },
    };
    preview['fontSize'] =
        double.tryParse(points.text.trim()) ?? source['fontSize'];
    if (changes.containsKey('fontSize')) {
      preview['_fontOverrides']['fontSize'] = preview['fontSize'];
    }
    if (characterMode) {
      final draft = {...changes};
      if (draft.containsKey('fontSize')) {
        draft['fontSize'] =
            double.tryParse(points.text.trim()) ?? source['fontSize'];
      }
      preview = previewCharacterFont(
        widget.sample,
        source,
        characterSelection,
        draft,
      );
    }
    return AlertDialog(
      title: const Text('Font'),
      content: SizedBox(
        width: 570,
        child: ConstrainedBox(
          constraints: BoxConstraints(
            maxHeight: (MediaQuery.sizeOf(context).height - 210).clamp(
              180.0,
              620.0,
            ),
          ),
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  key: const Key('font-summary'),
                  '${widget.fonts.length} ${widget.fonts.length == 1 ? 'cell' : 'cells'}${common.values.any((v) => v == null) ? ' · Mixed formatting' : ''}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                if (widget.allowCharacterSelection) ...[
                  CheckboxListTile(
                    key: const Key('font-character-mode'),
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    controlAffinity: ListTileControlAffinity.leading,
                    title: const Text('Format selected characters'),
                    value: characterMode,
                    onChanged: (v) => setCharacterMode(v == true),
                  ),
                  if (characterMode) ...[
                    draftTextField(
                      key: const Key('font-character-selection'),
                      controller: characters,
                      readOnly: true,
                      minLines: 1,
                      maxLines: 3,
                      style: const TextStyle(fontSize: 16),
                      decoration: InputDecoration(
                        labelText: 'Select the text to format',
                        helperText:
                            '${selectedSheetFontRuns(sheetFontRuns(widget.sample, sourceFont), characterSelection).map((r) => r.text).join().characters.length} characters selected · selecting again resets the draft',
                      ),
                    ),
                    const SizedBox(height: 8),
                  ],
                ],
                const SizedBox(height: 12),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: draftTextField(
                        key: const Key('font-family'),
                        controller: name,
                        autofocus: true,
                        decoration: InputDecoration(
                          labelText: 'Font family',
                          hintText: 'Mixed — unchanged',
                          helperText: installedCatalog
                              ? 'Search installed fonts or enter a name'
                              : 'Enter a font family name',
                        ),
                        onChanged: (v) => setProperty(
                          'fontName',
                          v.trim().isEmpty ? null : v.trim(),
                        ),
                      ),
                    ),
                    const SizedBox(width: 16),
                    SizedBox(
                      width: 110,
                      child: draftTextField(
                        key: const Key('font-size'),
                        controller: points,
                        keyboardType: const TextInputType.numberWithOptions(
                          decimal: true,
                        ),
                        decoration: const InputDecoration(
                          labelText: 'Size (points)',
                          hintText: 'Mixed',
                          helperText: '1–409',
                        ),
                        onChanged: (v) => setProperty(
                          'fontSize',
                          v.trim().isEmpty ? null : double.tryParse(v) ?? v,
                        ),
                      ),
                    ),
                  ],
                ),
                SizedBox(
                  height: characterMode ? 56 : 92,
                  child: filtered.isEmpty
                      ? const Center(child: Text('No matching font'))
                      : ListView.builder(
                          itemCount: filtered.length,
                          itemExtent: 30,
                          itemBuilder: (context, i) => InkWell(
                            onTap: () => chooseFamily(filtered[i]),
                            child: Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 8,
                                vertical: 5,
                              ),
                              child: Text(
                                filtered[i],
                                style: TextStyle(
                                  fontFamily: filtered[i],
                                  fontSize: 13,
                                ),
                              ),
                            ),
                          ),
                        ),
                ),
                if (missing)
                  const Padding(
                    padding: EdgeInsets.only(top: 6),
                    child: Text(
                      'This font is not installed. A substitute is used on this Mac.',
                      style: TextStyle(fontSize: 12, color: Color(0xff946200)),
                    ),
                  ),
                Wrap(
                  children: [
                    toggle('bold', 'Bold'),
                    toggle('italic', 'Italic'),
                    toggle('strike', 'Strikethrough'),
                  ],
                ),
                Row(
                  children: [
                    Expanded(
                      child: choice('underline', 'Underline', underlines),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: choice('verticalAlign', 'Position', verticals),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                draftTextField(
                  key: const Key('font-color'),
                  controller: color,
                  decoration: InputDecoration(
                    labelText: 'Font color',
                    hintText: 'Mixed — unchanged',
                    helperText: 'Automatic or #RRGGBB',
                    suffixIcon: PopupMenuButton<String>(
                      tooltip: 'Choose font color',
                      icon: const Icon(Icons.palette_outlined),
                      onSelected: chooseColor,
                      itemBuilder: (_) => [
                        for (final entry in palette.entries)
                          PopupMenuItem(
                            value: entry.key,
                            child: Row(
                              children: [
                                Container(
                                  width: 14,
                                  height: 14,
                                  color:
                                      sheetFontColor(entry.key) ??
                                      const Color(0xff242424),
                                ),
                                const SizedBox(width: 10),
                                Text(entry.value),
                              ],
                            ),
                          ),
                      ],
                    ),
                  ),
                  onChanged: (v) => setProperty(
                    'color',
                    v.trim().isEmpty
                        ? null
                        : v.trim().toLowerCase() == 'automatic'
                        ? 'automatic'
                        : v.trim().toUpperCase(),
                  ),
                ),
                const SizedBox(height: 12),
                const Text('Preview', style: TextStyle(fontSize: 12)),
                const SizedBox(height: 6),
                Container(
                  height: characterMode ? 64 : 80,
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  decoration: BoxDecoration(
                    color: Colors.white,
                    border: Border.all(color: const Color(0xffdfe3e8)),
                  ),
                  child: SheetFontText(
                    widget.sample.isEmpty ? 'AaBbCc 123' : widget.sample,
                    font: preview,
                    key: const Key('font-preview'),
                  ),
                ),
                if (characterMode || common.values.any((v) => v == null))
                  Padding(
                    padding: EdgeInsets.only(top: 6),
                    child: Text(
                      characterMode
                          ? 'Formatting applies to the highlighted characters.'
                          : 'Preview uses the first cell. Unchanged options keep each cell’s formatting.',
                      style: const TextStyle(fontSize: 12),
                    ),
                  ),
                if (error != null)
                  Padding(
                    padding: const EdgeInsets.only(top: 10),
                    child: Text(
                      error!,
                      key: const Key('font-error'),
                      style: const TextStyle(color: Colors.red),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => setState(() {
            changes.clear();
            name.text = common['fontName'] as String? ?? '';
            points.text = common['fontSize'] == null
                ? ''
                : fontPoints(common['fontSize'] as num);
            color.text = common['color'] as String? ?? '';
            error = null;
          }),
          child: const Text('Reset changes'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('apply-cell-font'),
          onPressed: changes.isEmpty ? null : apply,
          child: const Text('Apply'),
        ),
      ],
    );
  }

  @override
  void dispose() {
    characters.dispose();
    name.dispose();
    points.dispose();
    color.dispose();
    super.dispose();
  }
}
