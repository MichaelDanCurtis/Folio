import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'document_page_layout.dart';

class DocumentPagePreview extends StatefulWidget {
  final Map snapshot;
  const DocumentPagePreview({required this.snapshot, super.key});
  @override
  State<DocumentPagePreview> createState() => _DocumentPagePreviewState();
}

class _DocumentPagePreviewState extends State<DocumentPagePreview> {
  DocumentPageLayout? layout;
  String? error;
  bool started = false;
  int page = 0;
  double? zoom;
  final horizontal = ScrollController(), vertical = ScrollController();
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (started) return;
    started = true;
    final locale = Localizations.maybeLocaleOf(context);
    Future<void>(() {
      if (!mounted) return;
      try {
        final result = DocumentPageLayout.build(
          widget.snapshot,
          locale: locale,
        );
        if (!mounted) {
          result.dispose();
          return;
        }
        setState(() => layout = result);
      } catch (e) {
        if (mounted) {
          setState(() => error = e is FormatException ? e.message : '$e');
        }
      }
    });
  }

  @override
  void dispose() {
    layout?.dispose();
    horizontal.dispose();
    vertical.dispose();
    super.dispose();
  }

  void go(int value) {
    setState(() => page = value.clamp(0, layout!.pages.length - 1));
    if (vertical.hasClients) vertical.jumpTo(0);
  }

  @override
  Widget build(BuildContext context) => Dialog(
    insetPadding: const EdgeInsets.all(18),
    clipBehavior: Clip.antiAlias,
    child: SizedBox(
      width: 1200,
      height: 900,
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            child: Wrap(
              spacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                const Text(
                  'Page preview',
                  style: TextStyle(fontSize: 20, fontWeight: FontWeight.w600),
                ),
                if (layout != null) ...[
                  IconButton(
                    key: const Key('page-preview-previous'),
                    tooltip: 'Previous page',
                    onPressed: page > 0 ? () => go(page - 1) : null,
                    icon: const Icon(Icons.chevron_left),
                  ),
                  Text(
                    'Page ${page + 1} of ${layout!.pages.length}',
                    key: const Key('page-preview-count'),
                  ),
                  IconButton(
                    key: const Key('page-preview-next'),
                    tooltip: 'Next page',
                    onPressed: page + 1 < layout!.pages.length
                        ? () => go(page + 1)
                        : null,
                    icon: const Icon(Icons.chevron_right),
                  ),
                  Text('Section ${layout!.pages[page].section + 1}'),
                  DropdownButton<double>(
                    key: const Key('page-preview-zoom'),
                    value: zoom ?? 0,
                    underline: const SizedBox.shrink(),
                    items: const [
                      DropdownMenuItem(value: 0, child: Text('Fit page')),
                      DropdownMenuItem(value: .5, child: Text('50%')),
                      DropdownMenuItem(value: .75, child: Text('75%')),
                      DropdownMenuItem(value: 1, child: Text('100%')),
                      DropdownMenuItem(value: 1.5, child: Text('150%')),
                      DropdownMenuItem(value: 2, child: Text('200%')),
                    ],
                    onChanged: (value) =>
                        setState(() => zoom = value == 0 ? null : value),
                  ),
                  if (layout!.notices.isNotEmpty)
                    TextButton.icon(
                      key: const Key('page-preview-details'),
                      icon: const Icon(Icons.info_outline, size: 18),
                      label: Text('Layout details (${layout!.notices.length})'),
                      onPressed: () => showDialog<void>(
                        context: context,
                        builder: (context) => AlertDialog(
                          title: const Text('Page layout details'),
                          content: SizedBox(
                            width: 520,
                            child: SingleChildScrollView(
                              child: Column(
                                mainAxisSize: MainAxisSize.min,
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  for (final notice in layout!.notices)
                                    Padding(
                                      padding: const EdgeInsets.only(
                                        bottom: 12,
                                      ),
                                      child: Text(notice),
                                    ),
                                ],
                              ),
                            ),
                          ),
                          actions: [
                            TextButton(
                              onPressed: () => Navigator.pop(context),
                              child: const Text('Close'),
                            ),
                          ],
                        ),
                      ),
                    ),
                ],
                TextButton(
                  key: const Key('page-preview-close'),
                  onPressed: () => Navigator.pop(context),
                  child: const Text('Back to document'),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          Expanded(
            child: error != null
                ? Center(
                    child: Padding(
                      padding: const EdgeInsets.all(24),
                      child: Text(error!),
                    ),
                  )
                : layout == null
                ? const Center(child: CircularProgressIndicator())
                : LayoutBuilder(
                    builder: (context, viewport) => paper(viewport),
                  ),
          ),
          const Padding(
            padding: EdgeInsets.all(8),
            child: Text(
              'Previewed page values do not change the saved document.',
              style: TextStyle(fontSize: 12, color: Color(0xff606b79)),
            ),
          ),
        ],
      ),
    ),
  );
  Widget paper(BoxConstraints viewport) {
    final current = layout!.pages[page];
    final scale =
        zoom ??
        math
            .min(
              (viewport.maxWidth - 48) / current.size.width,
              (viewport.maxHeight - 48) / current.size.height,
            )
            .clamp(.1, 2.0);
    final width = current.size.width * scale,
        height = current.size.height * scale;
    final painted = CustomPaint(
      key: ValueKey('page-preview-paper-$page'),
      size: current.size,
      painter: _PagePainter(current),
    );
    final accessible = Semantics(
      label: 'Page ${page + 1}. ${current.visibleText}',
      child: RepaintBoundary(child: painted),
    );
    final scaled = Transform.scale(
      scale: scale,
      alignment: Alignment.topLeft,
      child: accessible,
    );
    final sized = SizedBox(
      width: width,
      height: height,
      child: OverflowBox(
        alignment: Alignment.topLeft,
        minWidth: current.size.width,
        maxWidth: current.size.width,
        minHeight: current.size.height,
        maxHeight: current.size.height,
        child: scaled,
      ),
    );
    final verticalScroll = SingleChildScrollView(
      controller: vertical,
      padding: const EdgeInsets.all(24),
      child: Center(child: sized),
    );
    final horizontalScroll = SingleChildScrollView(
      controller: horizontal,
      scrollDirection: Axis.horizontal,
      child: SizedBox(
        width: math.max(viewport.maxWidth, width + 48),
        height: viewport.maxHeight,
        child: verticalScroll,
      ),
    );
    return ColoredBox(
      color: const Color(0xffe8ebef),
      child: Scrollbar(
        controller: horizontal,
        notificationPredicate: (n) => n.metrics.axis == Axis.horizontal,
        child: horizontalScroll,
      ),
    );
  }
}

class _PagePainter extends CustomPainter {
  final LaidOutDocumentPage page;
  _PagePainter(this.page);
  @override
  void paint(Canvas canvas, Size size) {
    canvas.save();
    canvas.clipRect(Offset.zero & size);
    canvas.drawRect(Offset.zero & size, Paint()..color = Colors.white);
    for (final placement in page.content) {
      placement.paint(canvas);
    }
    if (!page.blank) {
      for (final band in page.columnBands) {
        band.paint(canvas);
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _PagePainter oldDelegate) =>
      oldDelegate.page != page;
}
