# Datum Calc: project memory

> **Read this at the start of each session** (`CLAUDE.md` says so). It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Calc: engineering calculation sheets with real units, readable maths, checks (PASS/FAIL with utilisation) and a printable calc sheet with a title block |
| **Current version** | **0.1.1** (released 2026-10-28). Earlier: 0.1.0 |
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
crates/core/src/parse.rs       lexer and Pratt parser: assignments, user functions, ranges, check, solve … for x in a .. b, plot … for x in a .. b; units after numbers; character spans for errors
crates/core/src/eval.rs        values (SI + dimension + abs temperature + vector), operators with dimension rules, built-in functions and constants, the sheet top to bottom (RegionOut: LaTeX, result, unit choices, check, plot, error), Variables, number formatting
crates/core/src/latex.rs       expression tree → LaTeX (fractions, roots, subscripts, Greek, units in roman)
crates/core/src/solve.rs       Brent's method after a 200-step scan for a sign change
crates/core/src/tests.rs       every unit factor vs NIST, temperatures, dimension errors, display, formatting, parser, functions, vectors, checks, solve (Colebrook), plot, LaTeX, a whole pump sheet
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
ui/src/sheet.tsx               the page canvas: title block, regions, the region editor
ui/src/calc.tsx                KaTeX (cached), MathView (maths, result, unit menu, PASS/FAIL, error with the marked span, warning), PlotView (Chart.js, family style), VarsPanel
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
| 0.1.1 | `release.yml` ran once on a push trigger to register it (tag pushes are blocked from the session); later releases use workflow_dispatch | A workflow needs one run before GitHub lists it |

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
| 0.1.2 | Table region; image region; units list; print polish | 🔜 |
| 0.1.3 | The 10 calc packs with their tests; File › New from pack; CSV export; screenshots; README; site.json | ⬜ |
| 0.1.4 | Fixes from trying every pack; a Getting Started sheet; polish | ⬜ |
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
- Engine: `units.rs` (about 200 units, NIST factors, absolute temperatures, display ladders), `parse.rs` (Pratt parser, units after numbers, statements), `eval.rs` (dimensions, functions, constants, vectors, checks, solve, plot, Variables), `latex.rs`, `solve.rs` (Brent). Tests: every unit factor, temperatures, dimension errors, display, LaTeX, Colebrook by solve, a whole pump sheet; fuzz and speed hardening tests.
- Window: KaTeX maths with results, a unit menu on every result, PASS/FAIL badges with utilisation, errors with the wrong part underlined, Chart.js plots, the Variables panel, counts in the status bar; printed sheets show the same.
- Checked in the real app with a full pump sheet (results, check, a dimension error, plot and the solved operating point 115.5 m³/h = √(40/0.003), worked by hand).
- Checks: clippy clean, 23 engine tests, 50 UI tests.
- Released 0.1.1.

## 5. Known limits and next steps

- Energy and torque share a dimension, so a torque shows in J unless its unit is chosen (N·m in the unit menu).
- Vectors are shown as their first values and the last; there are no matrices yet (0.3).
- Image and table regions exist in the format but have no editor yet (0.1.2); the cut order if time runs short is image, table, plot.
- The installer is not code-signed (1.0).
