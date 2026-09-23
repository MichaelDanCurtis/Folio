import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/file_browser.dart';
import 'package:folio_native/src/recent_files.dart';

void main() {
  testWidgets(
    'changing previews ignores stale results and damaged copies cannot restore',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late RecentFiles recent;
      late RecoveryCatalog catalog;
      late String a, b;
      await tester.runAsync(() async {
        dir = await Directory.systemTemp.createTemp('folio-browser-test-');
        recent = RecentFiles('${dir.path}/recent.json');
        await recent.load();
        catalog = RecoveryCatalog('${dir.path}/Snapshots');
        a = await catalog.reserve();
        b = await catalog.reserve();
        await File(a).writeAsString('{}');
        await File(b).writeAsString('{}');
      });
      final pending = <String, Completer<Map<String, dynamic>>>{};
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (_) => FileBrowser(
                  recent: recent,
                  recovery: catalog,
                  inspect: (path) =>
                      (pending[path] = Completer<Map<String, dynamic>>())
                          .future,
                ),
              ),
              child: const Text('Browse'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Browse'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Recovery copies'));
      for (
        var i = 0;
        i < 100 && find.byKey(ValueKey(a)).evaluate().isEmpty;
        i++
      ) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 5)),
        );
        await tester.pump();
      }
      expect(find.byKey(ValueKey(a)), findsOneWidget);
      await tester.tap(find.byKey(ValueKey(a)));
      await tester.pump();
      await tester.tap(find.byKey(ValueKey(b)));
      pending[b]!.complete({
        'title': 'Selected copy',
        'document': 'Correct preview',
        'sheets': ['Sheet 1'],
      });
      await tester.pumpAndSettle();
      pending[a]!.complete({
        'title': 'Stale copy',
        'document': 'Do not show',
        'sheets': [],
      });
      await tester.pumpAndSettle();
      expect(find.text('Correct preview'), findsOneWidget);
      expect(find.text('Do not show'), findsNothing);
      await tester.tap(find.byKey(ValueKey(a)));
      await tester.pump();
      pending[a]!.completeError(const FormatException('Invalid snapshot'));
      await tester.pumpAndSettle();
      expect(
        find.textContaining('This copy could not be opened'),
        findsOneWidget,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.widgetWithText(FilledButton, 'Restore copy'),
            )
            .onPressed,
        isNull,
      );
      await tester.enterText(find.byType(TextField), 'not found');
      await tester.pumpAndSettle();
      expect(find.text('No recovery copies found.'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.runAsync(() => dir.delete(recursive: true));
    },
  );
}
