# Parity status register

This folder is the current requirements register for Word and Excel parity (ROADMAP §9). [PARITY.md](../../PARITY.md) remains the family map and the history of each release. [VALIDATION.md](../../VALIDATION.md) and `release/validation` hold the evidence. [STATUS.md](../STATUS.md) summarizes this register: counts by state, the ranked gap list, risks and the change log.

| File | Families |
|---|---|
| [desktop.md](desktop.md) | D01–D10, the shared desktop product |
| [word.md](word.md) | W01–W17 |
| [excel.md](excel.md) | X01–X19 |
| [CORPUS.md](CORPUS.md) | Fixture corpus and the comparison files still needed |
| [OFFICE_BASELINE.md](OFFICE_BASELINE.md) | Reference Office builds, platform and machine |

## Child requirements

Each family is split into child requirements. A child names one user outcome, with its commands and options, and is small enough to be in a single state. When part of a feature works and part does not, the two parts are separate children.

```markdown
## W07 Tables

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W07.01 | ... | M1 | Native verified | 0.15 [TABLES](../SOME_DOC.md) | Word comparison |
```

- **ID:** `<family>.<nn>`. IDs are permanent: never renumbered or reused. A child that no longer applies stays, with state `Retired` and the reason in its last column. A new child takes the next free number and gets a line in the STATUS.md change log.
- **M:** the target milestone, M1–M5 (ROADMAP §3–4).
- **State:** the highest state its recorded evidence supports:
  1. **Specified:** written down here; not built.
  2. **Implemented:** in the product, without dedicated automated evidence.
  3. **Automated verified:** engine, widget or file-reader tests cover it.
  4. **Native verified:** checked in a packaged macOS build (historical checks count, with the version noted).
  5. **Office compared:** the same scenario was run in the recorded Word/Excel baseline, and the differences are listed.
  6. **Accepted:** every applicable ROADMAP §2 gate passed.
- **Evidence:** the version that established the state, plus a link to its feature doc or VALIDATION section. `—` when there is none.
- **Gap / next step:** the next gate, or the missing work.

Don't raise a state from prose alone. A feature described as implemented, without tests or a native check, is "Implemented" at most. Until [OFFICE_BASELINE.md](OFFICE_BASELINE.md) records the reference builds, no child can be "Office compared".

`test/status_register_test.dart` checks:
- all 46 families have children;
- IDs are unique and well formed;
- states and milestones are valid;
- relative links resolve;
- the STATUS.md counts match these tables.

Update the counts in STATUS.md whenever a state changes.
