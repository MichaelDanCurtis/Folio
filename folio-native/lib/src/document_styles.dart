import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

q.DefaultTextBlockStyle nativeParagraphStyle(
  int? level, {
  q.DefaultStyles? defaults,
}) {
  final styles = defaults ?? documentEditorStyles();
  return (switch (level) {
        1 => styles.h1,
        2 => styles.h2,
        3 => styles.h3,
        4 => styles.h4,
        5 => styles.h5,
        6 => styles.h6,
        _ => styles.paragraph,
      }) ??
      documentEditorStyles().paragraph!;
}

q.DefaultStyles documentEditorStyles() {
  q.DefaultTextBlockStyle style(
    double size, {
    double top = 0,
    double bottom = 12,
    bool bold = false,
  }) => q.DefaultTextBlockStyle(
    TextStyle(
      fontFamily: 'Aptos',
      fontWeight: bold ? FontWeight.bold : FontWeight.normal,
      fontFamilyFallback: const ['Calibri', 'Arial'],
      fontSize: size,
      height: 1.4,
      color: const Color(0xff242424),
    ),
    const q.HorizontalSpacing(0, 0),
    q.VerticalSpacing(top, bottom),
    const q.VerticalSpacing(0, 0),
    null,
  );
  return q.DefaultStyles(
    paragraph: style(16),
    h1: style(32, bottom: 18),
    h2: style(24, top: 18),
    h3: style(29 / 2 * 4 / 3, top: 14),
    h4: style(20, top: 8, bottom: 0, bold: true),
    h5: style(18, top: 8, bottom: 0, bold: true),
    h6: style(16, top: 8, bottom: 0, bold: true),
    lists: q.DefaultListBlockStyle(
      style(16).style,
      const q.HorizontalSpacing(0, 0),
      const q.VerticalSpacing(0, 12),
      const q.VerticalSpacing(0, 12),
      null,
      null,
    ),
  );
}
