# Datum Calc 0.1: plan

Written 2026-10-28 (session 1), from the hub brief `Edgar-Mendonca/Datum/briefs/DATUM-CALC.md`. Every decision there is already made with the user; this file turns it into a day plan. No open questions: build at once.

## What 0.1 is

Engineering calculation sheets. You type equations with real units; Datum Calc shows them as readable maths, works out the answers with units, checks them against limits (PASS/FAIL with a utilisation ratio) and prints a clean, referenced calc sheet with a title block.

- **The worksheet:** a page (A4 or Letter) of regions, top to bottom, evaluated in order. Region kinds: math, text, check, solve, plot, table, image.
- **Units:** a dimension vector (7 SI base exponents + an angle tag), about 200 units (SI with prefixes, US/imperial), absolute and difference temperatures, a chosen display unit per result and a sheet-wide unit system (SI or US).
- **Maths:** + − × ÷, powers, roots, functions (trig with degrees or radians, ln, log, exp, min, max, abs, round, if), ranges and vectors (`x = 0, 0.1 .. 10`), sum, mean, constants (g, π, e, R, σ), user functions, Greek and subscripts typed simply (`sigma_allow` → σ_allow).
- **Solve:** Brent's method in a range; "no root in range" is said clearly.
- **Calc packs:** ten ready, referenced sheets (File › New from pack), each also an engine test against a hand calculation.
- **Printing:** the calc sheet as PDF with a title block (project, calc number, title, by, checked, date, revision), "Page x of y" and the references at the end.
- **Exchange:** copy a result, export the inputs and results as CSV.

## Architecture (from the brief, §3)

| Part | Where | Notes |
| --- | --- | --- |
| Units | `crates/core/src/units.rs` | dimensions, the unit table, unit expressions (`kN*m`, `m^3/h`, `W/(m^2*K)`), display choice |
| Parser | `crates/core/src/parse.rs` | assignments, user functions, ranges, numbers with units, comparisons for checks; precise error positions |
| Evaluator | `crates/core/src/eval.rs` | top to bottom, a symbol table, vectors, errors per region (never stops the sheet) |
| Solve | `crates/core/src/solve.rs` | Brent's method |
| LaTeX | `crates/core/src/latex.rs` | the expression tree → LaTeX, so what is shown is what is computed |
| Sheet | `crates/core/src/sheet.rs` | regions, title block, page; `.dcalc` JSON with a format version; edit operations with undo/redo (like Plan's `doc.rs`) |
| Packs | `crates/core/src/packs.rs` + `crates/core/packs/*.dcalc` | embedded in the binary |
| Shell | `src-tauri` | commands, updater (`REPO = "Edgar-Mendonca/Datum-Calc"`), recovery, recent files |
| Window | `ui/src` | ribbon (Home · Insert · Math · Units · Check · View · Help, Format contextual), File page, page canvas, Variables panel, ? help, Options, print |

Plan's formula columns use the GridCraft crates (spreadsheet formulas), which have no units. Calc's grammar is different (units on numbers, assignments, ranges, user functions), so its parser is a small hand-written Pratt parser in the same compact style; the idea taken from Plan is "the engine computes, the window only shows" and the error-per-cell behaviour. No GridCraft dependency in Calc.

## The week

| Day | Work | Release |
| --- | --- | --- |
| 1 (Wed 28 Oct) | Read, plan; skeleton from Plan (renamed, amber, logo, updater, CI, release workflow); worksheet regions that save and open | **0.1.0** |
| 2 | Engine: units, parser, evaluator, LaTeX; 100+ unit tests | — |
| 3 | Worksheet UI: math, text and check regions with KaTeX; Variables panel; errors on the line | 0.1.1 |
| 4 | Solve and plot regions; table region; units list; print to PDF with title block | 0.1.2 |
| 5 | The 10 calc packs with their tests; File › New from pack; screenshots, README, `docs/site.json` | **0.1.3** (convention build) |
| 6–7 | Fixes from trying every pack; a "Getting started" sheet; polish | 0.1.4 |

If time runs short, cut in this order: image region, table region, plot region. **Never cut units, checks, packs or printing.**

## Risks

| Risk | What we do |
| --- | --- |
| Unit factors wrong (the whole point of the app) | A test for every unit factor against NIST SP 811 values; temperature tests (absolute and differences); dimension errors tested |
| KaTeX size and fonts in the installer | KaTeX is about 280 kB JS + 300 kB fonts; load only woff2 fonts; keep the bundle under 1 MB |
| Parser crashes on odd input | Hardening tests: random input never panics; damaged `.dcalc` opens as far as possible |
| Packs with wrong answers | Each pack is a test against a worked hand calculation written in the test with its source |
| Printing looks unprofessional | Title block and page frame drawn like a real calc sheet; checked by Xvfb screenshots of the printed PDF |
| One week | Small releases; the cut order above |
