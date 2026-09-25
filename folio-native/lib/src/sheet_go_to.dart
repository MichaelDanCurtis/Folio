// The name box and Go To (X02.04, X02.05): jump to a cell, range, whole rows
// or columns, a reference on another sheet, or a defined name.
import 'package:flutter/material.dart';

/// Where a reference or name leads.
class SheetTarget {
  /// The sheet named in the reference, or null for the current sheet.
  final String? sheetName;
  final int top, left, bottom, right;
  final bool wholeRows, wholeColumns;
  const SheetTarget(
    this.top,
    this.left,
    this.bottom,
    this.right, {
    this.sheetName,
    this.wholeRows = false,
    this.wholeColumns = false,
  });
  @override
  String toString() =>
      '${sheetName ?? ''}!$top,$left:$bottom,$right'
      '${wholeRows ? ' rows' : ''}${wholeColumns ? ' columns' : ''}';
}

const _maxRow = 1048575, _maxColumn = 16383;

int? _column(String letters) {
  if (letters.isEmpty || letters.length > 3) return null;
  var n = 0;
  for (final unit in letters.toUpperCase().codeUnits) {
    if (unit < 65 || unit > 90) return null;
    n = n * 26 + unit - 64;
  }
  return n - 1 <= _maxColumn ? n - 1 : null;
}

int? _row(String digits) {
  final n = int.tryParse(digits);
  return n != null && n >= 1 && n <= _maxRow + 1 ? n - 1 : null;
}

/// A cell, a row number or a column letter: (row, column), either null.
(int?, int?)? _end(String text) {
  final m = RegExp(r'^\$?([A-Za-z]*)\$?(\d*)$').firstMatch(text);
  if (m == null || (m[1]!.isEmpty && m[2]!.isEmpty)) return null;
  final column = m[1]!.isEmpty ? null : _column(m[1]!);
  final row = m[2]!.isEmpty ? null : _row(m[2]!);
  if ((m[1]!.isNotEmpty && column == null) ||
      (m[2]!.isNotEmpty && row == null)) {
    return null;
  }
  return (row, column);
}

/// Parses what a person types into the name box or Go To: `B7`, `A1:C9`,
/// `C:E`, `4:6`, `Sheet2!A1`, `'Q1 plan'!B2:B9`, or a defined name. [names]
/// are the workbook's defined names (`name`, `sheetId`, `refersTo`), and
/// [currentSheetId] picks a sheet-scoped name over a workbook one. Returns
/// null when the text isn't a reference or a name that refers to one.
SheetTarget? parseTarget(
  String input, {
  List<Map> names = const [],
  String? currentSheetId,
  int depth = 0,
}) {
  var text = input.trim();
  if (text.startsWith('=')) text = text.substring(1).trim();
  if (text.isEmpty || depth > 4) return null;
  String? sheet;
  final bang = text.lastIndexOf('!');
  if (bang > 0) {
    sheet = text.substring(0, bang);
    if (sheet.length > 1 && sheet.startsWith("'") && sheet.endsWith("'")) {
      sheet = sheet.substring(1, sheet.length - 1).replaceAll("''", "'");
    }
    text = text.substring(bang + 1);
  }
  final parts = text.split(':');
  if (parts.length <= 2) {
    final a = _end(parts.first);
    final b = parts.length == 2 ? _end(parts.last) : a;
    if (a != null && b != null) {
      final (r1, c1) = a;
      final (r2, c2) = b;
      final wholeColumns = r1 == null && r2 == null;
      final wholeRows = c1 == null && c2 == null;
      // A lone column letter or row number is a name, not a reference.
      final single = parts.length == 1 && (r1 == null || c1 == null);
      if (!single &&
          (wholeColumns ||
              wholeRows ||
              (r1 != null && c1 != null && r2 != null && c2 != null))) {
        int lo(int? x, int? y) => x == null ? 0 : (y == null || x < y ? x : y);
        int hi(int? x, int? y, int max) =>
            x == null ? max : (y == null || x > y ? x : y);
        return SheetTarget(
          lo(r1, r2),
          lo(c1, c2),
          hi(r1, r2, _maxRow),
          hi(c1, c2, _maxColumn),
          sheetName: sheet,
          wholeRows: wholeRows,
          wholeColumns: wholeColumns,
        );
      }
    }
  }
  if (sheet != null) return null;
  // A defined name: the one scoped to this sheet first, then the workbook's.
  Map? found;
  for (final name in names) {
    if ((name['name'] as String).toLowerCase() != text.toLowerCase()) continue;
    if (name['sheetId'] == currentSheetId && currentSheetId != null) {
      found = name;
      break;
    }
    if (name['sheetId'] == null) found ??= name;
  }
  if (found == null) return null;
  return parseTarget(
    found['refersTo'] as String,
    names: names,
    currentSheetId: currentSheetId,
    depth: depth + 1,
  );
}

/// The Go To dialog: defined names that refer to places, recent targets, and
/// a Reference field. Returns what to go to.
class SheetGoToDialog extends StatefulWidget {
  final List<Map> names;
  final List<String> recent;
  final String? currentSheetId;
  const SheetGoToDialog({
    super.key,
    required this.names,
    required this.recent,
    this.currentSheetId,
  });

  @override
  State<SheetGoToDialog> createState() => _SheetGoToDialogState();
}

class _SheetGoToDialogState extends State<SheetGoToDialog> {
  final controller = TextEditingController();
  String? error;

  /// Names that refer to places, then recent targets, each once.
  List<String> get places {
    final seen = <String>{};
    return [
      for (final place in [
        for (final name in widget.names)
          if (parseTarget(
                name['name'] as String,
                names: widget.names,
                currentSheetId: widget.currentSheetId,
              ) !=
              null)
            name['name'] as String,
        ...widget.recent,
      ])
        if (seen.add(place.toLowerCase())) place,
    ];
  }

  void submit() {
    final text = controller.text.trim();
    if (parseTarget(
          text,
          names: widget.names,
          currentSheetId: widget.currentSheetId,
        ) ==
        null) {
      setState(
        () => error = 'Enter a cell, a range or a name that refers to one.',
      );
      return;
    }
    Navigator.pop(context, text);
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Go To'),
    content: SizedBox(
      width: 320,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            height: 160,
            child: DecoratedBox(
              decoration: BoxDecoration(
                border: Border.all(color: const Color(0xffc7ccd0)),
              ),
              child: ListView(
                children: [
                  for (final place in places)
                    ListTile(
                      key: Key('go-to-$place'),
                      dense: true,
                      title: Text(place),
                      onTap: () => setState(() => controller.text = place),
                      onLongPress: () => Navigator.pop(context, place),
                    ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            key: const Key('go-to-reference'),
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              labelText: 'Reference',
              errorText: error,
              isDense: true,
            ),
            onSubmitted: (_) => submit(),
          ),
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('go-to-ok'),
        onPressed: submit,
        child: const Text('OK'),
      ),
    ],
  );
}
