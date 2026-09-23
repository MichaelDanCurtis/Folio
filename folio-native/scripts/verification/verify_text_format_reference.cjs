// The reference remains outside product dependencies: ssf@0.11.2.
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const ssf = require(path.resolve(process.argv[2]));
const snapshot = JSON.parse(fs.readFileSync(path.join(os.tmpdir(), 'folio-text-format-tests/text-formulas.json')));
const cells = snapshot.workspace.sheets[0].cells;
const values = snapshot.calculated.s;
const mismatches = [];
// SSF's upstream time-carry defect is covered by these explicit regressions.
const rounded = { B8: '12:00:00 PM', B9: '72:00:00' };
let compared = 0, carryRegressions = 0;
for (let r = 1; r <= 12; r++) {
  const address = `B${r}`;
  const match = /^=TEXT\(A\d+,"(.*)"\)$/.exec(cells[address]);
  if (!match) throw new Error(`Unexpected fixture formula at ${address}`);
  const format = match[1].replaceAll('""', '"');
  const expected = rounded[address] ?? ssf.format(format, values[`A${r}`]);
  if (rounded[address]) carryRegressions++; else compared++;
  if (values[address] !== expected) mismatches.push({address, format, actual: values[address], expected});
}
console.log(JSON.stringify({reference: 'SheetJS SSF 0.11.2, not Microsoft Excel', compared, carryRegressions, mismatches}, null, 2));
if (compared !== 10 || carryRegressions !== 2 || mismatches.length) process.exitCode = 1;
