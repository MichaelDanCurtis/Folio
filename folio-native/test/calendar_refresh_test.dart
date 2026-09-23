import 'dart:async';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/calendar_refresh.dart';

void main() {
  test(
    'calendar checks include rollback and timezone changes and coalesce work',
    () async {
      CalendarStamp stamp = (2024, 12, 31, -28800);
      var calls = 0;
      Completer<void>? pending;
      final refresh = CalendarRefresh(
        clock: () => stamp,
        onChange: () {
          calls++;
          return pending?.future ?? Future.value();
        },
      );
      await refresh.check();
      expect(calls, 0);
      stamp = (2025, 1, 1, -28800);
      pending = Completer<void>();
      final a = refresh.check(), b = refresh.check();
      expect(identical(a, b), isTrue);
      expect(calls, 1);
      stamp = (2025, 1, 1, -21600);
      pending.complete();
      await a;
      pending = null;
      await refresh.check();
      expect(calls, 2, reason: 'A clock change during refresh is not lost');
      stamp = (2024, 12, 31, -21600);
      await refresh.check();
      expect(calls, 3);
      refresh.dispose();
      stamp = (2025, 1, 2, -21600);
      await refresh.check();
      expect(calls, 3);
    },
  );

  test(
    'a failed refresh retries the same day, including synchronous throws',
    () async {
      CalendarStamp stamp = (2025, 1, 1, 0);
      var fail = true, calls = 0;
      final refresh = CalendarRefresh(
        clock: () => stamp,
        onChange: () {
          calls++;
          if (fail) throw StateError('refresh failed');
          return Future.value();
        },
      );
      stamp = (2025, 1, 2, 0);
      await expectLater(refresh.check(), throwsStateError);
      fail = false;
      await refresh.check();
      await refresh.check();
      expect(calls, 2);
      refresh.dispose();
    },
  );

  testWidgets(
    'periodic checks refresh only on change and stop after disposal',
    (tester) async {
      CalendarStamp stamp = (2025, 3, 9, -28800);
      var calls = 0;
      final refresh = CalendarRefresh(
        clock: () => stamp,
        onChange: () async {
          calls++;
        },
      );
      refresh.start();
      refresh.start();
      await tester.pump(const Duration(seconds: 30));
      expect(calls, 0);
      stamp = (2025, 3, 9, -25200);
      await tester.pump(const Duration(seconds: 30));
      expect(calls, 1);
      refresh.dispose();
      refresh.start();
      stamp = (2025, 3, 10, -25200);
      await tester.pump(const Duration(seconds: 60));
      expect(calls, 1);
    },
  );
}
