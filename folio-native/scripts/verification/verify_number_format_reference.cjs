// The reference is installed separately: npm install --prefix <dir> ssf@0.11.2 --ignore-scripts
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const ssf = require(path.resolve(process.argv[2]));
const cases = JSON.parse(fs.readFileSync(path.join(os.tmpdir(), 'folio-number-format-tests/reference-cases.json')));
const mismatches = []; let checked = 0, invalidDates = 0;
for (const item of cases) {
  if (item.warning && item.warning.includes('outside the workbook')) { invalidDates++; continue; }
  checked++;
  let expected;
  try { expected = ssf.format(item.format, item.value); } catch (e) { expected = 'ERROR: ' + e.message; }
  if (item.text !== expected) mismatches.push({...item, expected});
}
console.log(JSON.stringify({reference:'SheetJS SSF 0.11.2',checked,invalidDates,mismatches:mismatches.length,examples:mismatches.slice(0,20)},null,2));
if (checked !== 18020 || invalidDates !== 2000 || mismatches.length) process.exitCode = 1;
