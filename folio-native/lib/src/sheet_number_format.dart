/// Display results are calculated by the Rust spreadsheet formatter. Callers
/// without an engine snapshot show the value plainly instead of guessing an
/// Excel format using a different grammar.
String displayCell(dynamic value) {
  if (value == null) return '';
  if (value is bool) return value ? 'TRUE' : 'FALSE';
  if (value is num && value.isFinite && value == value.truncateToDouble()) {
    return value.toInt().toString();
  }
  return value.toString();
}

String formatCellValue(dynamic value, Map style, {Map? display}) =>
    display?['text'] as String? ?? displayCell(value);

Map<String, dynamic> numberDisplayStyle(Map style, Map? display) => {
  ...Map<String, dynamic>.from(style),
  ...Map<String, dynamic>.from(
    display?['conditionalStyle'] as Map? ?? const {},
  ),
  if (display?['conditionalStyle'] is Map)
    '_conditionalStyle': display!['conditionalStyle'],
  if (display?['color'] != null) '_numberFormatColor': display!['color'],
  if (display?['segments'] != null)
    '_numberFormatSegments': display!['segments'],
  if (display?['numberWidth'] != null) '_numberWidth': display!['numberWidth'],
};
