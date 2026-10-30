# Datum Calc: project memory

> **Read this at the start of each session** (`CLAUDE.md` says so). It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Calc: engineering calculation sheets with real units, readable maths, checks (PASS/FAIL with utilisation) and a printable calc sheet with a title block |
| **Current version** | **0.1.6** (released 2026-10-28). Earlier: 0.1.0, 0.1.1, 0.1.2, 0.1.3 (the convention build), 0.1.4, 0.1.5 |
| **Repo** | `Edgar-Mendonca/Datum-Calc` · working branch `claude/festive-fermi-ir7jyi` (the first branch pushed, so also the default branch) |
| **Stack** | Rust (engine `crates/core` + shell `src-tauri`), Tauri 2, WebView2, SolidJS + Vite UI in strict TypeScript (TSX), plain CSS with light/dark tokens, IBM Plex fonts; KaTeX for maths from 0.1.1; Chart.js for plots from 0.1.2. No Python |
| **Code size** | About 4,000 lines at 0.1.1: core 2,000 (with 600 lines of tests), shell 350, UI 1,600 TSX + CSS |
| **Installer** | NSIS `.exe`, per-user install: `DatumCalc-x.y.z-setup.exe` + `SHA256SUMS.txt` |
| **Brief** | [Datum/briefs/DATUM-CALC.md](https://github.com/Edgar-Mendonca/Datum/blob/HEAD/briefs/DATUM-CALC.md) (hub session 13); plan in [PLAN-0.1.md](PLAN-0.1.md) |
| **Reference repos** | `Edgar-Mendonca/Datum-Plan` (the reference implementation: ribbon, backstage, options, print, updater, help, roadmap, workflows, UI and hardening tests) · `Edgar-Mendonca/Datum` (the hub: FAMILY.md, MEMORY.md, brand, site) |

---

## 1. Architecture

```mermaid
flowchart LR
  subgraph UI["Window (WebView2) · ui/src/*.tsx → Vite → ui/dist"]
    Store[store.ts: signals for the sheet, selection,<br/>editing, zoom; act(op)]
    Rib[ribbon.tsx: tabs, symbol keys, Tell Me]
    Sheet[sheet.tsx: the page, title block, regions]
    Print[print.tsx: measured pages, title block, Page x of y]
  end
  subgraph Shell["src-tauri (Rust)"]
    Main[main.rs: commands, files, options,<br/>recovery, recent]
    Upd[update.rs: GitHub releases,<br/>SHA-256, silent install]
  end
  subgraph Core["crates/core (pure Rust, tested)"]
    Doc[sheet.rs: Sheet, Region, Op,<br/>Doc with undo/redo, .dcalc]
    Eng[units.rs → parse.rs → eval.rs<br/>latex.rs, solve.rs]
  end
  UI -- invoke --> Shell --> Core
```

### Files

```
crates/core/src/lib.rs         the engine's modules
crates/core/src/sheet.rs       Sheet (title block, page, units, digits, regions), Region (id, kind, text, unit, style, data), Op (Insert, Set, Delete, Move, Title, SheetSet, Ui, Batch), Doc (apply on a copy, undo/redo, open/save), tolerant .dcalc reader
crates/core/src/units.rs       dimensions (7 SI exponents + an angle tag), ~200 units with NIST factors, absolute temperatures (°C, °F with offsets), unit expressions, the display ladders (SI and US), labels (m³/h, W/(m²·K))
crates/core/src/parse.rs       lexer and Pratt parser: assignments, user functions, ranges, check, solve … for x in a .. b, plot … for x in a .. b; units after numbers and after brackets; 4j imaginary numbers; character spans for errors
crates/core/src/eval.rs        values (SI + dimension + abs temperature + vector + imaginary parts + the N·m reading), operators and complex arithmetic with dimension rules, built-in functions and constants, the sheet top to bottom (RegionOut: LaTeX, result, unit choices, check, plot, error), Variables, number formatting
crates/core/src/latex.rs       expression tree → LaTeX (fractions, roots, subscripts, Greek, units in roman)
crates/core/src/solve.rs       Brent's method after a 200-step scan for a sign change
crates/core/src/packs.rs       the nineteen calc packs (include_str! of crates/core/packs/*.dcalc), cards for File › New
crates/core/packs/getting-started.dcalc  the Getting Started sheet (packs::GETTING_STARTED, opened from File › Home and Help)
crates/core/examples/dump.rs   also `-- <pack id> [us] [numbers]` dumps a pack as the window's state (used to render every pack in the built window for a check by eye)
crates/core/examples/packs.rs  prints every pack worked out in SI and US units, for checking by eye
crates/core/packs/*.dcalc      the packs themselves (open and edit them in the app; `ui.pack` holds id, field, description)
crates/core/src/tests.rs       every unit factor vs NIST, temperatures, dimension errors, display, formatting, parser, functions, vectors, checks, solve (Colebrook), plot, LaTeX, a whole pump sheet; `tests::packs`: every pack against its hand calculation
crates/core/src/hardening.rs   FAMILY §11: failed edits change nothing, random edits keep invariants and undo restores, damaged files, speed gate
crates/core/examples/dump.rs   the UI test fixture (a small pump sheet as `state` returns it)
src-tauri/src/main.rs          commands: state, apply, history, new_sheet, open, save, save_file, pick_image, options, set_options, autosave, recovery, recover, recent, startup_file, version, open_url, quit, check_update, install_update
src-tauri/src/update.rs        the family updater (REPO = Edgar-Mendonca/Datum-Calc)
ui/index.html                  the icon sprite (amber)
ui/src/main.tsx                layout, sidebar (Setup, Sheet, Print), status bar, keys, startup, recovery
ui/src/store.ts                signals and act()/call()
ui/src/api.ts                  Tauri bridge and types
ui/src/ribbon.tsx              title bar, ribbon (Home · Insert · Math · Units · Check · View · Help, Format contextual), Tell Me
ui/src/backstage.tsx           File page: Home, Export, Options, Print, Roadmap, Updates
ui/src/sheet.tsx               the page canvas: title block, regions, the region editor, the table editor (local until Done/Esc/click outside), picture choice
ui/src/calc.tsx                KaTeX (cached), MathView (maths, result, unit menu, PASS/FAIL, error with the marked span, warning), PlotView (Chart.js, family style), TableView, ImageView, VarsPanel
ui/src/print.tsx               File › Print: pages packed from measured regions, title block on every page, references at the end
ui/src/dialogs.tsx             title block, getting started, glossary, shortcuts, save before closing
ui/src/help.tsx                the ? cards (HELP table)
ui/src/roadmap.tsx             File › Roadmap from docs/site.json
ui/src/cmds.ts                 commands for the ribbon, keys and Tell Me
tests_ui/run.cjs               Playwright UI tests against ui/dist with a mocked bridge
.github/workflows/ci.yml       clippy, tests, UI checks and tests, cargo deny, Windows + macOS
.github/workflows/release.yml  Windows installer + SHA256SUMS.txt, release notes from CHANGELOG
```

### Release flow

1. Bump `src-tauri/Cargo.toml` and `crates/core/Cargo.toml`; CHANGELOG section `## X.Y.Z — date`; update this file and `docs/site.json`.
2. Commit and push to the working branch.
3. Run `release.yml` (workflow_dispatch; it creates the tag `vX.Y.Z` and the GitHub Release with the installer and `SHA256SUMS.txt`).
4. Users get it from File › Updates; Datum Home installs it from the same release.

### How we work (the workflow, from Datum Plan)
1. **The user reviews the last release** and sends screenshots with bugs and wishes.
2. **Diagnose first.** Reproduce in the test harness or the real app before changing code.
3. **Bigger asks get a plan file** (`docs/PLAN-x.y.md`). For 0.1 the brief already answered every question, so the plan was written and building started at once.
4. **Engine first** (`crates/core`, with a unit test), then the UI.
5. **Check in the real app** under Xvfb (build `target/debug/datum-calc`, start it on `:99`, drive it with xdotool, screenshot with `import -window root`).
6. **Add a UI test** for every bug and feature (`tests_ui/run.cjs`).
7. **Run the checks:** clippy `-D warnings`, `cargo test --workspace`, `npm run check:ui`, the UI tests, `cargo deny check`.
8. **Release** (above), then watch CI and the release run.
9. **Report to the user** in plain language: what changed, what was checked, what is not done.

---

## 2. Decisions

| Version | Decision | Why |
| --- | --- | --- |
| 0.1.0 | Every product decision comes from the hub brief (session 13): name, amber accent, `.dcalc`, identifier, stack, KaTeX, ribbon tabs, logo, the week plan and the cut order | Decided with the user for the convention; no questions before building |
| 0.1.0 | The shell, ribbon, backstage, options, help, roadmap, updater, workflows and tests are copied from Datum Plan and renamed | FAMILY §12: copy, don't reinvent |
| 0.1.0 | Regions are a flat list in reading order (no free x/y placement) | A calc is read top to bottom; order is the evaluation order; simpler editing, printing and tests |
| 0.1.0 | Each region keeps the text the user typed; results, LaTeX and errors are computed from it, never stored | What is printed is always what the engine computes from the text |
| 0.1.0 | Calc's parser is its own small Pratt parser, not Plan's GridCraft formula engine | GridCraft evaluates spreadsheet formulas without units; Calc needs units on numbers, assignments, ranges and user functions. Plan's lesson kept: the engine computes, errors stay on their own line |
| 0.1.0 | The `.dcalc` reader opens damaged files field by field and drops only the regions it cannot read; ids are renumbered when duplicated | FAMILY §11 hardening: damaged files open as far as possible |
| 0.1.0 | Printing packs regions measured at the printed width into pages, never cutting a region; references (style Reference) print at the end | A calc sheet must look professional; equations split over pages are hard to check |
| 0.1.0 | The region editor stays open when the ribbon is clicked (Math and Units keys type into it) | The symbol keys would otherwise end the edit before typing |
| 0.1.0 | Page text stays dark on a light page in the dark theme | The page is paper; it prints black on white |
| 0.1.1 | Angle is a soft tag: it never makes values incompatible, but picks rad/s, rpm or degrees for display | An angle is a ratio; rpm ↔ rad/s works with rad = 1, and sin(30 deg) needs no special case |
| 0.1.1 | °C and °F values are absolute temperatures (`abs`); a difference of two is plain kelvin; adding a difference keeps it absolute; a mean stays absolute; a difference asked in °C shows as Δ°C | Engineers mix both; the classic mistake (°C × factor with the offset) must not happen |
| 0.1.1 | After a number, the first word is a unit if it is one; after that `*` and `/` continue the unit only for units that are not names on the sheet; `2*t` is never a unit | Readable input (`120 m^3/h`) without surprises when a name is also a unit symbol |
| 0.1.1 | σ is not a built-in constant (it is the usual name for stress); Stefan–Boltzmann is `σ_SB`. Constants (g, π, e, R, σ_SB, N_A, k_B, c_0) give way to names on the sheet | The brief lists σ; a silent constant behind an undefined stress would be dangerous |
| 0.1.1 | Utilisation is demand over capacity: lhs/rhs for ≤ and <, rhs/lhs for ≥ and > | A check reads the same whichever way it is written |
| 0.1.1 | Solve and plot exist already in 0.1.1 (the engine had them); 0.1.2 brings tables, pictures and the units list | Engine first; the cut order (image, table, plot) still holds |
| 0.1.1 | The window shows results only from the engine (`calc` in every `Out`); the whole sheet is worked out after every change | Simple and fast enough (1,500 lines well under a second in release builds; a speed gate tests it) |
| 0.1.1 | KaTeX and Plex fonts are bundled as woff2 only (a Vite transform drops woff/ttf) | Smaller installer; WebView2 reads woff2 |
| 0.1.2 | A table is columns (name, unit, optional formula) over rows of typed cells; each column becomes a vector; formula columns are evaluated with the columns to their left | Pump curves and data-sheet values are tables; vectors plug into interp, sum, max and plots |
| 0.1.2 | The table editor keeps its edits local and saves them as one change when it closes | Every save re-renders the sheet; typing from cell to cell must not lose focus, and undo should undo the whole edit |
| 0.1.2 | After a unit, `*` and `/` never read the constants (g, e, R, c_0), a function's parameters or a `for` variable as units | 0.1.1 read `kg/m^3*g` as grams (found when checking the table formula in the real app) |
| 0.1.3 | Packs are plain .dcalc files embedded with include_str!, each with a test comparing its results with hand values worked out independently (in the test comments) before running | The brief: every pack is a test; the files stay editable in the app |
| 0.1.3 | Text after `#` on a line is its description (shown beside it, printed) | A calc sheet describes its inputs; packs needed it |
| 0.1.3 | Pack units are set per region where the automatic choice would mislead (kN·m for moments, N·m for torque, kVA/kvar) | Energy and torque share a dimension; apparent power shares watts |
| 0.1.3 | Pack references cite textbooks and standards (White, Karassik, Colebrook, Brown, Incropera, Roark, Gere, Shigley, Chapman; IAPWS-IF97, ASME VIII-1 UG-27, ISO 898-1/724, IEC 60034-1) | The brief: each pack cites its textbook or open standard |
| 0.1.4 | An input literal shows in its typed unit when that unit belongs to the sheet's system (or to both); otherwise the ladder converts it | 8356 cm⁴ must not read 83.56×10⁶ mm⁴; a US sheet should still show US units |
| 0.1.4 | Numbers ≥ 10⁴ are rounded to the significant figures and grouped with thin spaces | 274863 showed six figures with digits = 4 |
| 0.1.4 | Getting Started is a sheet (like a pack), not only a dialog | People learn by changing a working example |
| 0.1.6 | `Doc::save` writes `name.dcalc.saving` and renames it over the sheet; `Doc::open` refuses files over 64 MB and reads bytes lossily; `normalize` cuts regions (5,000), text (20,000), pictures (8 MB), table rows (2,000) and columns (50); edits beyond them fail | A crash mid-save must never damage a sheet; a hostile file must not exhaust memory |
| 0.1.6 | ErrorBoundary around each region, the sheet view, the Variables panel and each printed region; utilisation formatted by `ratio()` that accepts anything | A UI test with malformed engine output showed the panel's `toFixed` could blank the whole window |
| 0.1.6 | A check's left side reads in the unit its name is shown in (`Env::shown`) | 0.0095 ≤ 0.03 for a percentage check |
| 0.1.6 | An input keeps its typed unit when the other system has no ladder for it; electrical units and cP/cSt are neutral; US head is BTU/lb | ohm/km became SI base units on a US sheet; cP became Pa·s |
| 0.1.6 | The print summary of checks is a pseudo-region (id −1) measured and packed like the others, saved in `sheet.ui.print.summary` | Reuses the page packing; a long summary moves the sheet down instead of overlapping |
| 0.1.5 | Lines are limited to 60 nested levels and 1,000 symbols, trees deeper than 400 are refused, and the sheet is worked out on its own 64 MB-stack thread | Thousands of brackets overflowed the stack (an abort on any build); the release build aborts on any panic, so limits come first and the thread is a second guard |
| 0.1.5 | Results must be finite and unit powers within ±60; unit powers in a unit expression above 60 are not read as units | 1e308×10 showed nothing; m^999 was silently m^127; i8 saturation made −128 whose abs() panicked (found by the new fuzz test) |
| 0.1.5 | The angle tag is dropped by roots and fractional powers when it does not divide evenly | ∛ of a torque from P/n (rad⁻¹) failed in the shaft pack |
| 0.1.5 | Substituted values use the unit each name is shown in on its own line (`Env::shown`) | Base units (m²·kg/s²) for a torque read badly next to its N·m line |
| 0.1.5 | The fuzz test builds random sheets from the grammar (numbers with units, names, operators, functions, checks, solve, plot, ranges, tables), 600 sheets per run in CI; 6,000 × 3 seeds and a release run before this release | Random characters rarely reach the evaluator; grammar-shaped input found two real panics at once |
| 0.1.1 | `release.yml` ran once on a push trigger to register it (tag pushes are blocked from the session); later releases use workflow_dispatch | A workflow needs one run before GitHub lists it |
| 0.1.7 | Windows publisher "Datum"; copyright stays "© 2026 Edgar Mendonca" | Family-wide rule (FAMILY §9) |
| 0.1.8 | Complex values are `Val.im` beside `Val.v` (empty when real) and become real again when the imaginary part is round-off (j² = −1) | One value type keeps vectors, units and errors working for both; tests compare strings like "-1" |
| 0.1.8 | `4j`/`4i` (letter touching the number) is always imaginary; `j`/`i` alone are constants a name on the sheet replaces; results always use j | Same as the built-in constants (g, e); j is the electrical habit |
| 0.1.8 | A unit after brackets applies to the bracket: `(3 + 4j) Ω` (`E::Unit`), not when the word is one of the sheet's names | `3 + 4j Ω` adds a number to ohms, an error that would surprise every electrical user |
| 0.1.8 | Polar form is a sheet setting (View › Polar, `Sheet.polar`), not per result | One reading per sheet, like Numbers and Figures |
| 0.1.8 | `Val.nm`: a force (or load per length) times a length, a plain factor of it or a sum with it shows in N·m; energy with an angle tag (P/ω) uses the torque ladder | Issue 2: torque came out in J; no pack changed, as packs pinned their units |
| 0.1.9 | Recovery copies are per window: `recovery/<pid>-<time>.dcalc/.json`, with `<id>.lock` held (`File::try_lock`) while the window runs; a copy whose lock is free is offered, newest first; 0.1.8's `recovery.dcalc` too | Two windows shared one copy and deleted each other's; the OS frees the lock when a window crashes, so no process list is needed |
| 0.1.9 | Dialog `no` runs only on the cancel button; × and Esc just close | Discard must delete the copy, × means "not now" |
| 0.1.9 | No utilisation for checks on absolute temperatures | A ratio of kelvins (0.91 for 30 °C ≤ 60 °C) reads as a margin it is not |
| 0.1.9 | rad/s not added to the menu of Hz results | 50 Hz would read as 50 rad/s; the Hz/rpm mix-up is already a known limit, not to be widened |

## 3. Roadmap

✅ done · 🔜 next · ⬜ later

| Version | Item | Status |
| --- | --- | --- |
| 0.1.0 | Shell from Plan (ribbon, File page, Options, help, Tell Me, themes), updater, CI and release workflows, logo | ✅ |
| 0.1.0 | Sheet document: regions, title block, page, undo/redo, `.dcalc` with a format version, recovery, recent files | ✅ |
| 0.1.0 | Print with the title block on every page, Page x of y, references at the end | ✅ |
| 0.1.0 | Hardening tests (failed edits, random edits, damaged files, speed gate) and UI tests | ✅ |
| 0.1.1 | Engine: units (about 200, NIST factors), parser, evaluator, LaTeX, solve (Brent), plot sampling; 23 test functions with about 450 assertions | ✅ |
| 0.1.1 | Worksheet: KaTeX maths, results with units and a unit menu, checks (PASS/FAIL, utilisation), solve and plot regions, Variables panel, errors on the line | ✅ |
| 0.1.2 | Table region (with formula columns), picture region (insert, paste, size), units/constants/functions list, print of tables and pictures | ✅ |
| 0.1.3 | The 10 calc packs with their tests; File › New from pack; notes after #; Copy Result, Copy as LaTeX; CSV export; screenshots; README; site.json | ✅ |
| 0.1.4 | Getting Started sheet; inputs keep their units; grouped numbers; packs convert in US units | ✅ |
| 0.1.5 | Hardening (limits, finite results, fuzz from the grammar), equations with their numbers, checks list, three more packs (shaft, tank, thermal expansion) | ✅ |
| 0.1.6 | Safe saving and file limits, UI error guards, packs 14–16 (pipe wall, column, cable drop), print summary of checks, figures per sheet, Duplicate and Insert Above | ✅ |
| 0.1.7 | Windows lists the publisher as Datum (family-wide change) | ✅ |
| 0.1.8 | Visitors' requests: complex numbers for AC circuits (View › Polar, re/im/arg/conj/polar), torque in N·m, AC circuit pack (17) | ✅ |
| 0.1.9 | Recovery per window (two sheets open at once), Discard that deletes, packs 18–19 (insulated pipe, fillet weld), upright j, no utilisation for absolute temperatures | ✅ |
| 0.1.10 | Fixes from trying every pack on a clean Windows PC and from the convention | 🔜 |
| 0.2 | Materials from Datum Atlas, more packs, unit menus on results, symbolic simplify | ⬜ |
| 0.3 | Matrices and linear systems, ODE solve, tables from CSV | ⬜ |
| 0.4 | Programs inside a region, templates, a company title block | ⬜ |
| 1.0 | Validation report, user guide, signed installer | ⬜ |

## 4. Session log

### 2026-10-28: Session 1 (plan + 0.1.0)
- Read the hub brief, CLAUDE.md, MEMORY.md, FAMILY.md and Datum Plan's code; wrote `docs/PLAN-0.1.md`.
- Built the skeleton from Plan: workspace, `crates/core` (sheet document with ops, undo/redo, tolerant reader, hardening tests), `src-tauri` (commands, options, recovery, recent, updater with `REPO = Edgar-Mendonca/Datum-Calc`), the window (ribbon with the seven Calc tabs and a Format contextual tab, File page, Options, Print, Roadmap, ? help, Tell Me, themes), amber tokens, the logo and Windows icons, CI and release workflows, docs.
- Checked in the real app under Xvfb: File page, a new sheet, typing equations (Enter starts the next), a text region, Ctrl+S to a `.dcalc`, reopening it from the command line, the print preview. Fixed: Plan's `.page`/`.rtext`/`.pwrap small` CSS rules clashed with the sheet; the title block grid; the editor closing when the ribbon was clicked.
- Checks: clippy clean, 4 hardening tests, 34 UI tests.
- Released 0.1.0 (the release workflow had to run once on a push trigger to be registered; tag pushes are blocked from the session).

### 2026-10-28: Session 1, continued (0.1.1, the engine)
- Engine: `units.rs` (about 200 units, NIST factors, absolute temperatures, display ladders), `parse.rs` (Pratt parser, units after numbers, statements), `eval.rs` (dimensions, functions, constants, vectors, checks, solve, plot, Variables), `latex.rs`, `solve.rs` (Brent). Tests: every unit factor, temperatures, dimension errors, display, LaTeX, Colebrook by solve, a whole pump sheet; `tests::packs`: every pack against its hand calculation; fuzz and speed hardening tests.
- Window: KaTeX maths with results, a unit menu on every result, PASS/FAIL badges with utilisation, errors with the wrong part underlined, Chart.js plots, the Variables panel, counts in the status bar; printed sheets show the same.
- Checked in the real app with a full pump sheet (results, check, a dimension error, plot and the solved operating point 115.5 m³/h = √(40/0.003), worked by hand).
- Checks: clippy clean, 23 engine tests, 50 UI tests.
- Released 0.1.1.

### 2026-10-28: Session 1, continued (0.1.2, tables and pictures)
- Engine: table regions (columns → vectors, units per column, formula columns), names defined by tables in the Variables panel; the constant/parameter fix for units after numbers.
- Window: table view and editor, picture regions (insert, paste, size, caption), the units/constants/functions list, printing of both.
- Checked in the real app: a pump curve table (editing a cell, Enter down, Done) and its formula column in kW; the print preview over two pages.
- Checks: clippy clean, 24 engine tests, 63 UI tests.
- Released 0.1.2.

### 2026-10-28: Session 1, continued (0.1.3, the convention build)
- Ten calc packs with hand values worked out first (independent Python evaluation of the same formulas) and a test each: all ten matched on the first run.
- Notes after # on a line; File › New with the packs grouped by field; right-click menu (Copy Result, Copy as LaTeX, Mark as Input); CSV export of inputs, results and checks.
- Screenshots (1440×900) of a pack, the printed sheet and the units in the real app; README and site.json updated.
- Checks: clippy clean, 36 engine tests (11 for the packs), 67 UI tests, cargo deny.
- Released 0.1.3.

### 2026-10-28: Session 1, continued (0.1.4)
- The user asked to stop after 0.1.3 and assess, then to build 0.1.4 first.
- Went through every pack in SI and US units (`--example packs`): inputs now keep their typed units, large numbers are rounded and grouped, packs pin units only for moments, torque, kVA and kvar.
- The Getting Started sheet (with its own test) opened from File › Home and Help.
- Checked in the real app: the sheet opens and works out with no errors; the updater found 0.1.3 as the latest release.
- Released 0.1.4. Not tested: a clean Windows PC, the real Save as PDF dialog, an update from one release to the next on Windows.

### 2026-10-28: Session 1, continued (0.1.5, a bigger release)
- The user was unwell and asked for a self-checked, hardened, larger release (three or four versions in one).
- Hardening: a probe of extreme lines found a stack overflow (deep brackets), a range-count overflow, unit powers silently capped at 127 and inf results shown as blank; the new grammar fuzz test found an i8 negate overflow. All fixed with limits and tests; 108,000 random sheets in debug and a full release-build run passed.
- Features: View › Numbers (substituted equations, saved and printed), the checks list in the side panel, packs 11–13 (shaft, tank, thermal expansion) with hand values first. The shaft pack found the angle-tag root bug.
- Self-check: all 14 sheets rendered in the built window in SI with Numbers and in US units (no errors, warnings, KaTeX failures or overflowing lines), a look by eye at the new ones; the real app under Xvfb for Numbers on screen and in print, a damaged and a non-sheet file, and the dark theme.
- Checks: clippy clean, 44 engine tests, 71 UI tests, cargo deny.
- Released 0.1.5. Still not tested: a clean Windows PC, the real Save as PDF dialog, an update from one release to the next on Windows.

### 2026-10-28: Session 1, continued (0.1.6)
- The user asked to keep going the same way: self-checked, hardened, several features in one release.
- Hardening: atomic save (temporary file and rename) with a test that a failed save leaves the sheet untouched; file size and content limits; lossy reading of damaged bytes; every pack saved, reopened and damaged 300 times; ErrorBoundary guards (a UI test with malformed engine output showed the Variables panel could blank the window; `ratio()` and the guards fixed it); 72,000 more random sheets and a release-build run.
- Features: packs 14–16 (pipe wall thickness, Euler column, cable voltage drop) with hand values first; print summary of checks; View › Figures; Duplicate (Ctrl+D) and Insert Above (Ctrl+Shift+M).
- Self-check: the packs in SI and US (`--example packs`) found a percentage check reading 0.0095 ≤ 0.03, ohm/km turning into base units and cP into Pa·s on a US sheet, US head in ft²/s² and a pinned kN on the Euler load; all fixed. All 17 sheets rendered in the built window in SI with Numbers and in US units without errors or overflow; the real app checked for the pack list, the print summary with a FAIL and the page count.
- Checks: clippy clean, 49 engine tests, 77 UI tests, cargo deny.
- Released 0.1.6. Still not tested: a clean Windows PC, the real Save as PDF dialog, an update from one release to the next on Windows.

### 2026-10-29: Session 1, continued (0.1.7)
- Family-wide change: the Windows publisher is "Datum" (`bundle.publisher` in `tauri.conf.json`); the copyright stays "© 2026 Edgar Mendonca". No other file writes the publisher. Noted in `docs/FAMILY.md`.
- The convention fixes move to 0.1.8.
- Released 0.1.7.

### 2026-10-30: Session 1, continued (0.1.8)
- The user asked to keep going with 0.1.8 the same way. Two visitors had opened issues: complex numbers for AC circuits (#1) and N·m as a unit of torque (#2); both went into 0.1.8, and the convention fixes moved to 0.1.9.
- Torque: N·m, kN·m, N·mm, MN·m, lbf·ft, lbf·in, kip·ft in the unit menu; a force times a length reads in N·m automatically. Every pack's output in SI and US was compared before and after: no change.
- Complex numbers: `4j`, `j`, units after brackets, complex arithmetic and functions, polar form, real-only errors elsewhere; pack 17 (AC circuit) checked against a hand calculation.
- Self-check: all 18 sheets rendered in the built window (SI with Numbers, and US) without errors or overflow; the AC sheet looked at by eye in both forms, which found KaTeX spacing a minus after ∠ as subtraction (fixed with a group). A probe of edge cases found 0^(1j) giving 0 and re() of a temperature losing °C (both fixed).
- Checks: clippy clean, 52 engine tests, 80 UI tests, cargo deny; 24,000 more random sheets (three seeds in release, one in debug) with complex values in the grammar.
- Released 0.1.8. Still not tested: a clean Windows PC, the real Save as PDF dialog, an update from one release to the next on Windows.

### 2026-10-30: Session 1, continued (0.1.9)
- The user asked to keep going with 0.1.9. No new visitor issues; reading the shell for what a clean Windows PC would meet found that two windows (two sheets double-clicked in Explorer) shared one recovery copy and deleted each other's, and that Discard in the recovery box never deleted the copy.
- Recovery per window with a lock file; tried in the real app under Xvfb: crashed window's copy offered and recovered (sheet intact), a running window's copy never offered, kill -9 then restart, Discard, a 0.1.8 copy, two windows at once.
- Packs 18–19 (insulated pipe, fillet weld) with hand values first; the weld pack renamed to f_vw_d, F_w_Rd so subscripts read as in EN 1993-1-8.
- Upright j in equations; no utilisation for checks on absolute temperatures (found writing the insulation pack).
- Checks: clippy clean, 54 engine tests, 82 UI tests, cargo deny; all 20 sheets rendered in SI with Numbers and in US without errors or overflow; 18,000 more random sheets in release.
- Released 0.1.9. Still not tested: a clean Windows PC, the real Save as PDF dialog, an update from one release to the next on Windows.

## 5. Known limits and next steps

- Energy and torque share a dimension: a force times a length (and a power over a speed) shows in N·m, a power times a time in J; m·g·h shows in N·m too (choose J in the unit menu).
- Complex values work in arithmetic and in sqrt, exp, ln, abs, re, im, arg, conj, polar, sum and mean; trigonometric functions, checks, solve, plots, ranges and tables take real values only. Results show j, also when i was typed.
- Hz, rad/s and rpm are all 1/s (the SI convention), so f = 50 Hz shown in rpm reads 477.5, not 3000; write ω = 2*pi*f or n = 3000 rpm explicitly.
- Two windows that crash together leave two copies; the newest is offered first, the other at the next start.
- Vectors are shown as their first values and the last; there are no matrices yet (0.3).
- Tables hold values and formula columns, not free spreadsheet formulas per cell.
- The installer is not code-signed (1.0).
