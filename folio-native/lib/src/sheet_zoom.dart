// Worksheet zoom (X05.03): 10–400%, kept per sheet as `zoom` (the XLSX
// `sheetView/@zoomScale`).
import 'package:flutter/material.dart';

const minZoom = 10, maxZoom = 400;

/// The steps Command/Ctrl with + and − move through.
const zoomSteps = [10, 25, 50, 75, 100, 125, 150, 200, 300, 400];

int clampZoom(num percent) => percent.round().clamp(minZoom, maxZoom);

/// The next step above (or below, with [up] false) [percent].
int stepZoom(int percent, {required bool up}) {
  if (up) {
    return zoomSteps.firstWhere((s) => s > percent, orElse: () => maxZoom);
  }
  return zoomSteps.lastWhere((s) => s < percent, orElse: () => minZoom);
}

/// Zoom to Selection: the largest zoom at which a selection of the given
/// size (at 100%) fits the viewport, as Excel computes it.
int zoomToFit({
  required double selectionWidth,
  required double selectionHeight,
  required double viewportWidth,
  required double viewportHeight,
}) {
  if (selectionWidth <= 0 || selectionHeight <= 0) return 100;
  final scale = [
    viewportWidth / selectionWidth,
    viewportHeight / selectionHeight,
  ].reduce((a, b) => a < b ? a : b);
  return clampZoom((scale * 100).floorToDouble());
}

/// Lays [child] out at the viewport size divided by [zoom] and scales it up,
/// so painting and hit testing both follow the zoom.
class ZoomedViewport extends StatelessWidget {
  final double zoom;
  final Widget Function(BoxConstraints unscaled) builder;
  const ZoomedViewport({super.key, required this.zoom, required this.builder});

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final unscaled = BoxConstraints.tight(
        Size(constraints.maxWidth / zoom, constraints.maxHeight / zoom),
      );
      if (zoom == 1) return builder(constraints);
      return ClipRect(
        child: OverflowBox(
          alignment: Alignment.topLeft,
          minWidth: unscaled.maxWidth,
          maxWidth: unscaled.maxWidth,
          minHeight: unscaled.maxHeight,
          maxHeight: unscaled.maxHeight,
          child: Transform.scale(
            scale: zoom,
            alignment: Alignment.topLeft,
            child: builder(unscaled),
          ),
        ),
      );
    },
  );
}

/// The status-bar zoom control: −, a slider, + and the percentage, which
/// opens preset zooms and Zoom to Selection.
class SheetZoomControl extends StatefulWidget {
  final int zoom;

  /// Called with `commit` false while the slider moves, and true when a zoom
  /// is chosen.
  final void Function(int percent, {required bool commit}) onZoom;
  final VoidCallback? onZoomToSelection;
  const SheetZoomControl({
    super.key,
    required this.zoom,
    required this.onZoom,
    this.onZoomToSelection,
  });

  @override
  State<SheetZoomControl> createState() => _SheetZoomControlState();
}

class _SheetZoomControlState extends State<SheetZoomControl> {
  double? dragging;

  @override
  Widget build(BuildContext context) {
    final shown = dragging?.round() ?? widget.zoom;
    const muted = TextStyle(fontSize: 9, color: Color(0xff5f6368));
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        _button('zoom-out', Icons.remove, 'Zoom out', () {
          widget.onZoom(stepZoom(widget.zoom, up: false), commit: true);
        }),
        SizedBox(
          width: 110,
          height: 24,
          child: SliderTheme(
            data: SliderTheme.of(context).copyWith(
              trackHeight: 2,
              thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 5),
              overlayShape: SliderComponentShape.noOverlay,
            ),
            child: Slider(
              key: const Key('zoom-slider'),
              min: minZoom.toDouble(),
              max: maxZoom.toDouble(),
              value: shown.toDouble().clamp(
                minZoom.toDouble(),
                maxZoom.toDouble(),
              ),
              onChanged: (v) {
                setState(() => dragging = v);
                widget.onZoom(clampZoom(v), commit: false);
              },
              onChangeEnd: (v) {
                setState(() => dragging = null);
                widget.onZoom(clampZoom(v), commit: true);
              },
            ),
          ),
        ),
        _button('zoom-in', Icons.add, 'Zoom in', () {
          widget.onZoom(stepZoom(widget.zoom, up: true), commit: true);
        }),
        PopupMenuButton<int>(
          key: const Key('zoom-percent'),
          tooltip: 'Zoom',
          onSelected: (v) => v == 0
              ? widget.onZoomToSelection?.call()
              : widget.onZoom(v, commit: true),
          itemBuilder: (_) => [
            for (final step in zoomSteps.reversed)
              PopupMenuItem(
                key: Key('zoom-$step'),
                height: 30,
                value: step,
                child: Text('$step%'),
              ),
            if (widget.onZoomToSelection != null) ...[
              const PopupMenuDivider(),
              const PopupMenuItem(
                key: Key('zoom-selection'),
                height: 30,
                value: 0,
                child: Text('Zoom to Selection'),
              ),
            ],
          ],
          child: SizedBox(
            width: 38,
            child: Text('$shown%', textAlign: TextAlign.right, style: muted),
          ),
        ),
      ],
    );
  }

  Widget _button(String key, IconData icon, String tip, VoidCallback onTap) =>
      SizedBox(
        width: 20,
        height: 20,
        child: IconButton(
          key: Key(key),
          tooltip: tip,
          padding: EdgeInsets.zero,
          iconSize: 12,
          style: IconButton.styleFrom(
            tapTargetSize: MaterialTapTargetSize.shrinkWrap,
          ),
          onPressed: onTap,
          icon: Icon(icon),
        ),
      );
}
