# Phase 4: M0 Parity Inventory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the prose-heavy parity ledger with a finite, checkable inventory. All 46 families are split into bounded child requirements, each with a stable ID and an acceptance state. The register is tied to recorded evidence and ends in a ranked gap list, which the M1 delivery packages are chosen from.

**Exit gate (program plan and ROADMAP M0):**
- `folio-native/docs/STATUS.md` covers 46/46 families.
- Every existing feature maps to child IDs.
- There is a ranked gap list with effort ranges.
- The Office reference baseline and benchmark machine are recorded, or explicitly blocked on the person who has them.
- The highest-risk dependencies are evaluated.

**Architecture:**
- **The register is data in Markdown tables**, one file per product area under `folio-native/docs/status/`. It stays readable in review and on GitHub.
- **A Flutter test (`test/status_register_test.dart`) guards the register's integrity in CI.** It checks that:
  - every family has children;
  - IDs are unique and well formed;
  - states come from the ROADMAP's list;
  - evidence links resolve;
  - the summary counts in `STATUS.md` match the tables.

  That makes the "one concise current status register" (ROADMAP §9) something CI keeps honest, not prose that drifts.

**Scope boundary:** no product code changes. The only measurements are of things that already exist, such as formula catalog coverage and fixture coverage.

## Global Constraints

- **States:** Specified → Implemented → Automated verified → Native verified → Office compared → Accepted (ROADMAP §2). A child's state is the highest one its recorded evidence supports.
  - Historical native evidence (VALIDATION.md, release/validation archives) counts, with its version noted.
  - Nothing is "Office compared" yet: no direct Word/Excel comparison has been recorded.
  - Nothing is "Accepted".
- **Bounded children:** a child requirement names one user outcome with its commands and options. It is small enough to reach a single state. If part of a feature is done and part isn't, split it into two children rather than write "partial".
- **IDs:** `<FAMILY>.<nn>`, for example `W07.04`. IDs are never reused or renumbered; a retired child is marked "Retired" with a reason. New requirements get the next free number and appear in the change log.
- **No completion claims beyond the evidence.** Don't infer completion from prose that says "implemented" without tests or native checks (ROADMAP §9).
- The existing PARITY.md stays the family map and history. STATUS.md becomes the current register, and PARITY.md links to it.

## Register format

Each area file (`docs/status/desktop.md`, `word.md`, `excel.md`) has one section per family:

```markdown
## W07 Tables

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W07.01 | Insert a table with a chosen grid; ... | M1 | Native verified | 0.15 ([VALIDATION](../../VALIDATION.md#015-word-table-editing-2026-09-18)) | Word comparison |
```

- **M:** the target milestone (M1–M5), per ROADMAP §4.
- **Evidence:** a version plus a link to a VALIDATION section or feature doc, or `—`.
- **Gap / next step:** the next gate this child needs.

`STATUS.md` holds:
- per-family counts by state;
- the ranked gap list;
- the risk register;
- the baseline and corpus status;
- the change log.

---

### Task 1: Register format and integrity test

**Files:** `folio-native/docs/status/README.md`, `folio-native/test/status_register_test.dart`

- [ ] Document the format above, the state rules and the ID rules.
- [ ] The test parses every family table in `docs/status/*.md` and checks:
  - all 46 family IDs (D01–D10, W01–W17, X01–X19) are present, each with at least one child;
  - IDs are unique, match their family, and have two-digit sequence numbers;
  - every state is valid;
  - the milestone is M1–M5;
  - relative evidence links point to files that exist;
  - the counts table in `STATUS.md` equals the recomputed counts.

### Task 2: Shared desktop families D01–D10

- [ ] Decompose each family from ROADMAP S01–S07 and the PARITY entry. Map existing evidence (feature docs, VALIDATION sections) to children.

### Task 3: Word families W01–W17

- [ ] Decompose from ROADMAP W-A…W-H, the PARITY entries and the Word feature docs (`docs/*.md`).

### Task 4: Excel families X01–X19

- [ ] Decompose from ROADMAP X-A…X-J, the PARITY entries and the Excel feature docs.
- [ ] **Measure formula-catalog coverage** for X08: list IronCalc's implemented functions against Excel's function catalog. Record the count and the missing functions by category, so X08 has real numbers.

### Task 5: Baseline, corpus and benchmark records

- [ ] `docs/status/OFFICE_BASELINE.md`: the fields ROADMAP §2 requires (Word/Excel version, build, channel, OS, locale, fonts, extensions, licenses, Copilot). Blocked on the user, who has the Office installs; stated as such.
- [ ] Benchmark machine: link the Phase 1 record (`PERFORMANCE_BASELINE.md`, M4 Max) and list which ROADMAP §7 budgets are measured and which are not.
- [ ] `docs/status/CORPUS.md`:
  - index the existing fixtures (`test/fixtures`), with the child IDs each one exercises;
  - list the Office-created comparison files the M1 daily-workflow scenario needs, marked "needs Office".

### Task 6: Risk register

- [ ] For each ROADMAP §7 architecture risk, record what is known, what was measured or proved here, and the decision or proof still needed:
  - rich-text layout ceiling (Quill);
  - formula-engine coverage;
  - editable OOXML objects;
  - VBA/add-ins;
  - data connectors;
  - collaboration;
  - font licensing;
  - platforms.

### Task 7: Ranked gap list and STATUS.md

- [ ] Rank the gaps by M1 daily-workflow impact first, then by milestone. Give each an effort range (S ≤ 1 week, M 1–3 weeks, L 3–8 weeks, XL > 8 weeks) and a confidence (high/medium/low).
- [ ] Choose the first M1 delivery package from the top of the list.
- [ ] Write `STATUS.md`, link it from PARITY.md and ROADMAP.md, and update the program plan's Phase 4 row.
- [ ] Run the local CI steps.
