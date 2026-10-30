# Native checks, September 2026

Run 2026-09-26 on the owner's Mac: Mac16,5 (Apple M4 Max, 128 GiB), macOS 27.0 (26A428), on `main` at 3d198a6 (Phase 16). The app under test was the packaged `Folio Native.app` 0.82.0, unpacked fresh from `release/Folio Native.zip` (bundle `com.folio.native`, arm64, minimum macOS 12.0, ad-hoc signed, `codesign --verify --deep --strict` passes).

Summary: every automated suite passes. Eight register rows move to Native verified. Two native checks failed (D01.09 and D05.04). Every Word and Excel comparison is blocked, because Office is not installed on this Mac.

## 1. Toolchain

| Tool | Expected | Found |
|---|---|---|
| Xcode | — | 27.0 (27A266a), command-line tools present |
| Flutter / Dart | 3.44.7 / 3.12.2 | 3.44.7 / 3.12.2 (Homebrew cask) |
| Rust | 1.98.1 via rustup | 1.98.1. rustup auto-installed the pinned toolchain the first time `cargo +1.98.1` ran; nothing else was installed |
| python3 | any | 3.14.5 |

Two changes were needed to build with Xcode 27.0; the owner approved both:
- **Minimum macOS 10.15 → 12.0.** Xcode 27 accepts deployment targets from 12.0 to 27.0 only, and fails on 10.15.
- **Build arm64 only** (`ARCHS = arm64` in the three project configurations). Flutter 3.44.7 checks a universal framework with one `lipo <binary> -verify_arch arm64 x86_64` call. Xcode 27.0's lipo rejects that form ("-verify_arch requires exactly one input file") and accepts only one architecture per call. The x86_64 slice could not have worked anyway: the bundled `libfolio_core.dylib` and the Codex runtime are arm64 only.

## 2. Build and automated tests

| Step | Result |
|---|---|
| `cargo +1.98.1 test --release --lib` | 517 passed, 0 failed, 3 ignored |
| `flutter test` with `FOLIO_CORE_PATH` | 484 passed, 3 skipped, 0 failed. The seven platform tests that fail on Linux (file access, macOS menu variants) pass here |
| `scripts/build_macos.sh` | Builds; the manifest is verified for Folio 0.82.0; the Codex 0.154.0 runtime was fetched with `fetch_codex.py` (checksum verified) |

Notes:
- The first `flutter test` run failed 18 tests, because `rust/target/release/libfolio_core.dylib` dated from 2026-09-20. `cargo test --lib` does not rebuild the cdylib. After `cargo build --release` (the first step of `build_macos.sh`), everything passes. Run `cargo build --release` before `flutter test`.
- `release/Folio Native.app` fails `codesign --verify`, and the fresh zip does not. `build_macos.sh` copies the new app into the existing folder with `ditto`, which merges, so a `libswift_Concurrency.dylib` from an older build stays inside. `com.apple.FinderInfo` also comes back on three framework folders on this external volume. Test from the zip, or delete `release/Folio Native.app` before building.

## 3. Native scripts

| Script | Result |
|---|---|
| `test_macos_close.py` | Pass |
| `test_macos_file_menu.py` | Pass after an update. The script still expected the original ten-item File menu. It now also covers New Tab (⌘T), Open in New Tab (⌥⌘O), Close Tab (⌥⌘W), and Export PDF being hidden in workbooks |
| `test_macos_history_menu.py` | Pass |
| `test_macos_keyboard.py` | Pass (47 checks) |
| `test_macos_lifecycle.py` | Pass |

These scripts compile the production Swift sources with `xcrun swift` and use in-memory events; they do not launch the app.

## 4. Manual checks

Claude ran these through the macOS UI, working on scratch copies outside the repository. For the Quit prompt about the owner's existing sample tab, the owner chose Don't Save.

### Tabs (D01.07, D01.09)

| Check | Result | Notes |
|---|---|---|
| ⌘T opens a new tab and shows the tab strip | Pass | Keystroke and native menu item |
| ⌥⌘O opens a file in a new tab | Pass | The Open panel offers Word and Excel files only |
| Each tab keeps its own save state | Pass | Three tabs showed three different states; saving one did not change the others, and the file on disk had only that tab's edit |
| File and Edit menus act on the tab in front | Pass | Edit → Undo and File → Save from the menu bar changed only the tab in front |
| ⌥⌘W with unsaved changes asks to save | Pass | "Save changes to your files?" names the file; Cancel keeps the tab |
| Quit with unsaved tabs asks each tab in turn | Pass | Three prompts, one per tab. Don't Save and Save both behaved as chosen |
| Relaunch restores the tabs | Pass, with a note | All three tabs come back, with the tab in front restored. A file tab closed with Don't Save comes back as a blank workspace, not the file as last saved. The saved tab shows "Changes saved locally" |
| Relaunch restores the window position | **Fail** | Moved to x 956, y 527 before quitting; came back at x 1200, y 332 (centred). `MainFlutterWindow.awakeFromNib` calls `center()` after `setFrameAutosaveName("FolioMainWindow")`, which overrides the restored frame, so the autosaved frame is always the centred one (`1200 900 1440 928` on a 3840-pt display) |

Also observed: the tab strip and title bar say "Untitled workspace" even for a tab opened from `tab-open.docx`.

### PDF export (D05.04, D05.05)

The test document had Heading 1 and Heading 2, a hyperlink, an inserted PNG, Greek, Chinese and Japanese text, a footnote, a caption, a cross-reference and a table of contents. It was exported with File → Export PDF… and opened in Preview.

| Check | Result | Notes |
|---|---|---|
| Text can be selected and searched | Pass | Searching 中文测试 finds 1 match; the text selects; the text extracts with pypdf, non-Latin text included |
| Outline works | Pass | Preview's table of contents lists both headings |
| Link works | Pass | Preview offers to open https://github.com/MichaelDanCurtis/Folio |
| Tags | Pass | `/Lang en`, `/MarkInfo /Marked true`, structure tree Document › H1, P, H2, P, Figure (alt "Picture"), P; title shown |
| Fonts | Pass | Embedded subsets of ArialMT, ArialUnicodeMS and Arial-ItalicMT |
| Looks like the page preview | **Fail** | The page preview shows dot leaders in the TOC and the PDF does not. Both show the level-1 page number short of the right tab stop. Both leave out the footnote text at the foot of the page (known gap, W11.05) |

### References (W10, W11)

| Check | Result | Notes |
|---|---|---|
| Insert a footnote | Pass | Marker ¹ at the caret; saved as `footnoteReference` with `word/footnotes.xml` |
| Insert a caption | Pass | "Figure 1: …" in the Caption style, a `SEQ Figure \* ARABIC` field |
| Insert a cross-reference | Pass, with a note | "Figure 1" as `REF _Ref100000001 \h` with the hidden bookmark. After Insert, the editor does not get keyboard focus back, so typing is lost until you click the text |
| Insert a table of contents | Pass | A `TOC \o "1-3" \u` field in a Table of Contents content control, with a right dot-leader tab |
| Update Fields | Pass | Picks up a renamed heading. With the caption deleted, the reference shows "Error! Reference source not found." (W10.08) |
| Edit an existing footnote | Not checked | Clicking the marker needs pixel precision that remote control could not reach. W11.01 stays Automated verified |
| Save as DOCX, open in Word, compare | **Blocked** | Word is not installed. The DOCX is ready for the comparison: `native-check-references.docx`, rebuilt by the steps above |

### Spill and Name Manager (X09)

| Check | Result | Notes |
|---|---|---|
| `=SEQUENCE(3,3)` spills with an outline | Pass | 1–9 in A1:C3 with a blue outline; spilled cells show the formula. A value in the way gives #SPILL!, and Undo brings the spill back |
| Name Manager adds, edits and deletes | Pass | Added `TaxRate` (0.25) and `Double` (`=LAMBDA(x,x*2)`); edited TaxRate to 0.2; `=Double(TaxRate*100)` gives 40; deleting `Double` asks first, and the cell shows #NAME? |
| Save as XLSX, open in Excel | **Blocked** | Excel is not installed. What the file contains, checked by hand: function prefixes are right (`_xlfn.`, `_xleta.SUM`), and `TaxRate` is a defined name. Two defects: spills are written as plain formulas without dynamic-array metadata (no `cm` attribute, no `xl/metadata.xml`, no cached spilled values), so Excel will probably show `=@SEQUENCE(3,3)` as one value. And B5 keeps the cached value 40 after `Double` was deleted, where Folio shows #NAME? |

Also observed: after Enter, a small grey chip with the previous value sometimes shows in the active cell.

### Functions (X08)

These are Folio's results next to Excel's documented results; nothing was run in Excel.

| Formula | Folio | Excel (documented) |
|---|---|---|
| `=AGGREGATE(9,6,A1:C3)` over the 1–9 spill | 45 | 45 |
| `=BAHTTEXT(1234.5)` | หนึ่งพันสองร้อยสามสิบสี่บาทห้าสิบสตางค์ | same |
| `=FORECAST.ETS(13,SEQUENCE(12),SEQUENCE(12))` | 13 | 13 |
| `=GROUPBY({"a";"b";"a"},{1;2;3},SUM)` | a 4, b 2, Total 6 (spills) | same |

X08.03 stays Automated verified until it is compared with Excel.

## 5. Office baseline

| Field | Value |
|---|---|
| Word version and build | Not installed |
| Excel version and build | Not installed |
| macOS | 27.0 (26A428) |
| Language and region | English (en-US), region United States |
| Date and time | Short date M/d/yy (9/26/26), medium Sep 26, 2026; 12-hour time (h:mm a); Gregorian calendar; week starts Sunday; America/Los_Angeles |
| Numbers and currency | 1,234,567.89 (decimal `.`, grouping `,`); US dollar, −$1,234.50 |
| Measurement | US system |

Filled into the macOS column of [OFFICE_BASELINE](status/OFFICE_BASELINE.md). The baseline stays "not recorded" until Word and Excel are installed and their About boxes are recorded.

## Register changes

To Native verified: D01.07, D05.05, W10.03, W10.04, W10.05, W10.08, X09.02, X09.05.

Unchanged, with the finding in their gap column: D01.09 (window frame), D05.04 (TOC leaders). Unchanged, not fully checked: W11.01 (editing a note), X09.04 (LET), X08.03 (Excel comparison).

## Since the check

- [#19](https://github.com/MichaelDanCurtis/Folio/pull/19): the PDF now writes TOC dot leaders, and right-tab page numbers end at the stop. D05.04 needs a native recheck.
- [#20](https://github.com/MichaelDanCurtis/Folio/pull/20): XLSX export now writes dynamic arrays (`t="array"`, `cm`, `xl/metadata.xml`) with cached spilled values, and writes `#NAME?` for a formula whose name was deleted.

## Still to do on this Mac

1. Install Microsoft 365, record the Word and Excel builds, then open `native-check-references.docx` in Word and `native-check-sheet.xlsx` in Excel and compare.
2. Recheck D01.09 once the window frame fix is in.
3. Edit an existing footnote (W11.01).
