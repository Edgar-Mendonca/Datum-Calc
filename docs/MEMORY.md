# Datum Calc: project memory

> **Read this at the start of each session** (`CLAUDE.md` says so). It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Calc: engineering calculation sheets with real units, readable maths, checks (PASS/FAIL with utilisation) and a printable calc sheet with a title block |
| **Current version** | **0.1.0** (released 2026-10-28) |
| **Repo** | `Edgar-Mendonca/Datum-Calc` · working branch `claude/festive-fermi-ir7jyi` (the first branch pushed, so also the default branch) |
| **Stack** | Rust (engine `crates/core` + shell `src-tauri`), Tauri 2, WebView2, SolidJS + Vite UI in strict TypeScript (TSX), plain CSS with light/dark tokens, IBM Plex fonts; KaTeX for maths from 0.1.1; Chart.js for plots from 0.1.2. No Python |
| **Code size** | About 2,000 lines at 0.1.0: core 450 (with 150 lines of tests), shell 330, UI 1,300 TSX + CSS |
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
  end
  UI -- invoke --> Shell --> Core
```

### Files

```
crates/core/src/lib.rs         the engine's modules
crates/core/src/sheet.rs       Sheet (title block, page, units, digits, regions), Region (id, kind, text, unit, style, data), Op (Insert, Set, Delete, Move, Title, SheetSet, Ui, Batch), Doc (apply on a copy, undo/redo, open/save), tolerant .dcalc reader
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

## 3. Roadmap

✅ done · 🔜 next · ⬜ later

| Version | Item | Status |
| --- | --- | --- |
| 0.1.0 | Shell from Plan (ribbon, File page, Options, help, Tell Me, themes), updater, CI and release workflows, logo | ✅ |
| 0.1.0 | Sheet document: regions, title block, page, undo/redo, `.dcalc` with a format version, recovery, recent files | ✅ |
| 0.1.0 | Print with the title block on every page, Page x of y, references at the end | ✅ |
| 0.1.0 | Hardening tests (failed edits, random edits, damaged files, speed gate) and UI tests | ✅ |
| 0.1.1 | Engine: units (about 200, NIST factors), parser, evaluator, LaTeX; 100+ unit tests | 🔜 |
| 0.1.1 | Worksheet: KaTeX maths, results with units, checks (PASS/FAIL, utilisation), Variables panel, errors on the line | 🔜 |
| 0.1.2 | Solve (Brent) and plot regions; table region; units list; print polish | ⬜ |
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
- Released 0.1.0.

## 5. Known limits and next steps

- 0.1.0 keeps the text of each region; nothing is evaluated yet (0.1.1).
- Image, table, plot and solve regions exist in the format but have no editor yet (0.1.2); the cut order if time runs short is image, table, plot.
- The installer is not code-signed (1.0).
