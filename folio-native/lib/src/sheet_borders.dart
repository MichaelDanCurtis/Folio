// Cell borders and fill colour (X04.07, X04.08): the grid's painting of a
// cell's `border` style, and the Borders and Fill Color menus.
import 'package:flutter/material.dart';

/// Excel's border presets, in the order of its Borders menu.
const borderPresets = <(String, String)>[
  ('bottom', 'Bottom Border'),
  ('top', 'Top Border'),
  ('left', 'Left Border'),
  ('right', 'Right Border'),
  ('none', 'No Border'),
  ('all', 'All Borders'),
  ('outside', 'Outside Borders'),
  ('thickBox', 'Thick Box Border'),
];

/// Line styles Folio draws, with their menu labels.
const borderLineStyles = <(String, String)>[
  ('thin', 'Thin'),
  ('medium', 'Medium'),
  ('thick', 'Thick'),
  ('dashed', 'Dashed'),
  ('dotted', 'Dotted'),
  ('double', 'Double'),
  ('hair', 'Hair'),
];

/// The palette shared by Fill Color and the border colour: Excel's standard
/// colours.
const standardColors = <(String, String)>[
  ('#C00000', 'Dark Red'),
  ('#FF0000', 'Red'),
  ('#FFC000', 'Orange'),
  ('#FFFF00', 'Yellow'),
  ('#92D050', 'Light Green'),
  ('#00B050', 'Green'),
  ('#00B0F0', 'Light Blue'),
  ('#0070C0', 'Blue'),
  ('#002060', 'Dark Blue'),
  ('#7030A0', 'Purple'),
  ('#000000', 'Black'),
  ('#FFFFFF', 'White'),
];

Color? _hex(Object? value) {
  if (value is! String || !RegExp(r'^#[0-9A-Fa-f]{6}$').hasMatch(value)) {
    return null;
  }
  return Color(0xff000000 | int.parse(value.substring(1), radix: 16));
}

/// Width on screen for an Excel line style, at 100% zoom.
double borderWidth(String? style) => switch (style) {
  'hair' => 0.5,
  'medium' || 'mediumDashed' || 'mediumDashDot' || 'mediumDashDotDot' => 2,
  'thick' || 'double' => 3,
  null || 'none' => 0,
  _ => 1,
};

BorderSide borderSide(Object? side) {
  if (side is! Map) return BorderSide.none;
  final width = borderWidth(side['style'] as String?);
  if (width == 0) return BorderSide.none;
  return BorderSide(
    color: _hex(side['color']) ?? const Color(0xff000000),
    width: width,
  );
}

/// The border a cell's `border` style paints, or null when it has none.
/// Sides without a line fall back to [gridline] so the grid stays visible.
Border? cellBorder(Map? style, BorderSide gridline) {
  final border = style?['border'];
  if (border is! Map) return null;
  final top = borderSide(border['top']),
      left = borderSide(border['left']),
      bottom = borderSide(border['bottom']),
      right = borderSide(border['right']);
  if ([top, left, bottom, right].every((s) => s == BorderSide.none)) {
    return null;
  }
  return Border(
    top: top,
    left: left,
    bottom: bottom == BorderSide.none ? gridline : bottom,
    right: right == BorderSide.none ? gridline : right,
  );
}

/// A popup-menu entry showing colours as a grid of swatches, like Excel's
/// palette. Choosing one returns `valuePrefix` + the colour.
class SwatchGrid extends PopupMenuEntry<String> {
  final List<(String, String)> colors;
  final String valuePrefix;
  final String? selected;
  const SwatchGrid({
    super.key,
    required this.colors,
    this.valuePrefix = '',
    this.selected,
  });

  @override
  double get height => 36.0 * ((colors.length + 5) ~/ 6) + 8;

  @override
  bool represents(String? value) => false;

  @override
  State<SwatchGrid> createState() => _SwatchGridState();
}

class _SwatchGridState extends State<SwatchGrid> {
  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
    child: Wrap(
      spacing: 6,
      runSpacing: 6,
      children: [
        for (final (hex, name) in widget.colors)
          Tooltip(
            message: name,
            child: InkWell(
              key: Key('${widget.valuePrefix}$hex'),
              onTap: () => Navigator.pop(context, '${widget.valuePrefix}$hex'),
              child: Container(
                width: 30,
                height: 30,
                decoration: BoxDecoration(
                  color: Color(
                    0xff000000 | int.parse(hex.substring(1), radix: 16),
                  ),
                  border: Border.all(
                    color: widget.selected == hex
                        ? const Color(0xff107c41)
                        : const Color(0xffc7ccd0),
                    width: widget.selected == hex ? 2.5 : 1,
                  ),
                ),
              ),
            ),
          ),
      ],
    ),
  );
}
