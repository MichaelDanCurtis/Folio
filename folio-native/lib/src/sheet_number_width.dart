/// Shorter representations for General only. Format-section selection and the
/// unconstrained representation still belong to the engine. These candidates
/// never become cell values, clipboard data or saved number-format codes.
Iterable<String> generalWidthCandidates(num value, String original) sync* {
  if (!value.isFinite || value == 0) return;
  final seen = <String>{original};
  final parts = original.toUpperCase().split('E');
  final dot = parts.first.indexOf('.');
  final decimals = dot < 0 ? 0 : parts.first.length - dot - 1;
  if (parts.length == 1 && decimals > 0) {
    for (var places = decimals - 1; places >= 0; places--) {
      final candidate = _stripZeros(value.toStringAsFixed(places));
      // A nonzero value must not become zero merely because its cell is narrow.
      if (num.tryParse(candidate) != 0 && seen.add(candidate)) yield candidate;
    }
  }
  final precision = parts.length > 1 ? decimals - 1 : 10;
  for (var places = precision; places >= 0; places--) {
    final exponential = value.toStringAsExponential(places).split('e');
    final exponent = int.parse(exponential.last);
    final candidate =
        '${_stripZeros(exponential.first)}E${exponent < 0 ? '-' : '+'}'
        '${exponent.abs().toString().padLeft(2, '0')}';
    if (seen.add(candidate)) yield candidate;
  }
}

String _stripZeros(String text) {
  if (!text.contains('.')) return text;
  return text.replaceFirst(RegExp(r'0+$'), '').replaceFirst(RegExp(r'\.$'), '');
}
