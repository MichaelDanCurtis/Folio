import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

/// Lays [child] out at 1/[scale] of the available width and takes its size
/// times [scale], so the scroll extent, painting, hit testing and
/// showOnScreen all follow the zoom (W17.02).
class ScaledBox extends SingleChildRenderObjectWidget {
  final double scale;
  const ScaledBox({super.key, required this.scale, super.child});
  @override
  RenderScaledBox createRenderObject(BuildContext context) =>
      RenderScaledBox(scale);
  @override
  void updateRenderObject(BuildContext context, RenderScaledBox renderObject) {
    renderObject.scale = scale;
  }
}

class RenderScaledBox extends RenderProxyBox {
  RenderScaledBox(this._scale);
  double _scale;
  double get scale => _scale;
  set scale(double value) {
    if (value == _scale) return;
    _scale = value;
    markNeedsLayout();
  }

  Matrix4 get _transform => Matrix4.diagonal3Values(_scale, _scale, 1);

  @override
  void performLayout() {
    final c = constraints;
    BoxConstraints unscaled(BoxConstraints c) => BoxConstraints(
      minWidth: c.minWidth / _scale,
      maxWidth: c.maxWidth.isFinite ? c.maxWidth / _scale : double.infinity,
      minHeight: c.minHeight / _scale,
      maxHeight: c.maxHeight.isFinite ? c.maxHeight / _scale : double.infinity,
    );
    if (child == null) {
      size = c.smallest;
      return;
    }
    child!.layout(unscaled(c), parentUsesSize: true);
    size = c.constrain(child!.size * _scale);
  }

  @override
  Size computeDryLayout(BoxConstraints constraints) => constraints.constrain(
    (child?.getDryLayout(constraints / _scale) ?? Size.zero) * _scale,
  );

  @override
  double computeMinIntrinsicWidth(double height) =>
      (child?.getMinIntrinsicWidth(height / _scale) ?? 0) * _scale;
  @override
  double computeMaxIntrinsicWidth(double height) =>
      (child?.getMaxIntrinsicWidth(height / _scale) ?? 0) * _scale;
  @override
  double computeMinIntrinsicHeight(double width) =>
      (child?.getMinIntrinsicHeight(width / _scale) ?? 0) * _scale;
  @override
  double computeMaxIntrinsicHeight(double width) =>
      (child?.getMaxIntrinsicHeight(width / _scale) ?? 0) * _scale;

  @override
  void paint(PaintingContext context, Offset offset) {
    if (child == null) return;
    if (_scale == 1) {
      context.paintChild(child!, offset);
      return;
    }
    layer = context.pushTransform(
      needsCompositing,
      offset,
      _transform,
      (context, offset) => context.paintChild(child!, offset),
      oldLayer: layer is TransformLayer ? layer as TransformLayer : null,
    );
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      child != null &&
      result.addWithPaintTransform(
        transform: _transform,
        position: position,
        hitTest: (result, position) =>
            child!.hitTest(result, position: position),
      );

  @override
  void applyPaintTransform(RenderBox child, Matrix4 transform) {
    transform.multiply(_transform);
  }
}
