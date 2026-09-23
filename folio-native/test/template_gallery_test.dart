import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/template_gallery.dart';
import 'package:folio_native/src/templates.dart';
import 'package:folio_native/src/working_files.dart';

void main() {
  testWidgets(
    'template previews ignore stale results and failed previews cannot create',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late TemplateLibrary library;
      await tester.runAsync(() async {
        dir = await Directory.systemTemp.createTemp('folio-template-gallery-');
        library = TemplateLibrary(dir.path, WorkingFileAccess());
        await library.load();
        final a = await library.acquire(starterTemplates.first);
        final b = await library.acquire(starterTemplates.first);
        expect(a.path, isNot(b.path));
        expect(
          await File(a.path).readAsBytes(),
          await File(b.path).readAsBytes(),
        );
        await a.close();
        expect(await File(b.path).exists(), isTrue);
        await b.close();
        expect(await File(a.path).exists(), isFalse);
      });
      final pending = <String, Completer<Map<String, dynamic>>>{};
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (_) => TemplateGallery(
                  library: library,
                  preview: (entry) =>
                      (pending[entry.id] = Completer<Map<String, dynamic>>())
                          .future,
                  browse: (_) async => null,
                ),
              ),
              child: const Text('Templates'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Templates'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('template-letter')));
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('template-report')));
      await tester.pump();
      pending['report']!.complete({
        'document': 'Selected report',
        'fingerprint': 'second',
      });
      await tester.pumpAndSettle();
      pending['letter']!.complete({
        'document': 'Stale letter',
        'fingerprint': 'first',
      });
      await tester.pumpAndSettle();
      expect(find.text('Selected report'), findsOneWidget);
      expect(find.text('Stale letter'), findsNothing);
      expect(
        find.text('Edit template'),
        findsNothing,
        reason: 'Bundled starters cannot be edited in place',
      );
      await tester.tap(find.byKey(const ValueKey('template-letter')));
      await tester.pump();
      pending['letter']!.completeError(
        const FormatException('Invalid template'),
      );
      await tester.pumpAndSettle();
      expect(find.textContaining('could not be previewed'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, 'Create'))
            .onPressed,
        isNull,
      );
      await tester.enterText(find.byType(TextField), 'No matching template');
      await tester.pumpAndSettle();
      expect(find.text('No matching templates.'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.runAsync(() => dir.delete(recursive: true));
    },
  );
}
