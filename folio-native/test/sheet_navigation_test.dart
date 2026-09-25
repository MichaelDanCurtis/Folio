import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_navigation.dart';

void main() {
  // Rows in column A: 0–2 filled, a gap, 5–6 filled, a gap, 10 filled.
  final cells = {
    'A1': '1',
    'A2': '2',
    'A3': '3',
    'A6': 'x',
    'A7': 'y',
    'A11': '=A1',
    'B1': 'b',
    'A20': '',
  };
  bool noneHidden(int _) => false;
  int down(int p) => p < 99 ? p + 1 : p;
  int up(int p) => p > 0 ? p - 1 : p;

  int edge(int from, int direction, {bool Function(int)? hidden}) {
    final isHidden = hidden ?? noneHidden;
    int step(int p) {
      var at = p;
      do {
        final next = direction > 0 ? down(at) : up(at);
        if (next == at) return p;
        at = next;
      } while (isHidden(at));
      return at;
    }

    return dataEdge(
      from: from,
      direction: direction,
      populated: populatedAlong(cells, column: 0, hidden: isHidden),
      step: step,
      edge: direction > 0 ? 99 : 0,
    );
  }

  test('populated positions ignore empty values and other columns', () {
    expect(populatedAlong(cells, column: 0, hidden: noneHidden), [
      0,
      1,
      2,
      5,
      6,
      10,
    ]);
    expect(populatedAlong(cells, row: 0, hidden: noneHidden), [0, 1]);
  });

  test('Ctrl+Down walks runs, gaps and the sheet edge like Excel', () {
    expect(edge(0, 1), 2, reason: 'inside a run: to its last cell');
    expect(edge(2, 1), 5, reason: 'end of a run: to the next filled cell');
    expect(edge(5, 1), 6);
    expect(edge(6, 1), 10);
    expect(edge(10, 1), 99, reason: 'no data ahead: to the sheet edge');
    expect(edge(3, 1), 5, reason: 'from a blank: to the next filled cell');
    expect(edge(99, 1), 99, reason: 'at the edge already');
  });

  test('Ctrl+Up mirrors Ctrl+Down', () {
    expect(edge(10, -1), 6);
    expect(edge(6, -1), 5);
    expect(edge(5, -1), 2);
    expect(edge(2, -1), 0);
    expect(edge(0, -1), 0);
    expect(edge(50, -1), 10, reason: 'from a blank: to the previous filled');
  });

  test('hidden rows are skipped', () {
    // Hiding row 5 leaves row 6 as the start of the next run.
    expect(edge(2, 1, hidden: (r) => r == 5), 6);
    // Hiding the gap (rows 3–4) joins the two runs.
    expect(edge(0, 1, hidden: (r) => r == 3 || r == 4), 6);
  });

  test('last used cell spans the populated rows and columns', () {
    expect(lastUsedCell(cells), (10, 1));
    expect(lastUsedCell({}), (0, 0));
  });

  test('current region grows through touching cells, including diagonals', () {
    final data = {
      'B2': 'h1',
      'C2': 'h2',
      'B3': '1',
      'C3': '2',
      'D4': 'diag',
      'F9': 'far',
    };
    expect(currentRegion(data, 2, 1), (1, 1, 3, 3));
    expect(currentRegion(data, 8, 5), (8, 5, 8, 5));
    expect(currentRegion(data, 20, 20), isNull);
    // A blank cell touching data belongs to that data's region; one two
    // rows away does not (Ctrl+A there selects the sheet).
    expect(currentRegion(data, 3, 1), (1, 1, 3, 3));
    expect(currentRegion(data, 4, 1), isNull);
  });
}
