# Changelog

## 0.1.6 — 2026-10-28

Safer saving, three more calc packs, a summary of checks on the printed sheet, figures per sheet, Duplicate and Insert Above.

**More calc packs** (16 in all)
- Pipe wall thickness for internal pressure, with corrosion allowance and mill tolerance, against the nominal wall of the schedule.
- Column buckling (Euler): critical load, critical stress, slenderness, the load against the critical load with a safety factor, and whether Euler's formula applies.
- Cable voltage drop (three-phase): from resistance and reactance per km, as a percentage of the line voltage, against a limit.
- Each is checked against a hand calculation in the tests.

**Printing**
- Print › Summary of checks at the start: a box at the top of the first page lists every check with both values, the utilisation and PASS or FAIL, and says whether all pass. It is saved with the sheet.

**Editing**
- View › Figures sets how many significant figures this sheet shows (3 to 6); the engine always works at full precision.
- Duplicate (Ctrl+D) copies a region below it with its style and unit; Insert Above (Ctrl+Shift+M) adds an equation above the selected one. Both are on Home › Arrange.

**Results that read better**
- A check reads in the unit its value is shown in: 0.95 % ≤ 3 %, not 0.0095 ≤ 0.03.
- An input whose unit has no counterpart in the other unit system keeps its own unit (0.193 ohm/km on a US sheet). Electrical units and cP are used the same way in both systems.
- Specific energy and head on a US sheet show in BTU/lb.

**Sturdier**
- Saving writes to a temporary file first and then replaces the sheet in one step, so a crash or a full disk never leaves a half-written sheet.
- Files over 64 MB are refused; bytes that are not text are replaced, so a damaged sheet still opens as far as it can. Very long texts, huge pictures and oversized tables in a file are cut to the limits; an edit beyond them is refused with a message.
- If one region (or the Variables panel) ever cannot be drawn, it shows a short note and the rest of the window keeps working. A utilisation that is not a number shows nothing instead of stopping the panel.
- Every pack is saved, reopened and damaged hundreds of times in the tests; another 72,000 random sheets ran without a failure.

## 0.1.5 — 2026-10-28

A bigger release: equations shown with their numbers, three more calc packs, a list of every check, and a round of hardening that found and fixed four ways the engine could fail.

**Equations with their numbers** (View › Numbers)
- Each equation is shown a second time with the values put in, as on a hand calculation: P = ρ g Q H/η = 998 kg/m³ · 9.807 m/s² · 120 m³/h · 45 m / 0.75 = 19.57 kW. Each value appears in the unit it is shown in on its own line. The setting is saved with the sheet and printed.

**More calc packs** (13 in all)
- Shaft torsion: torque from power and speed, shear stress, the smallest diameter for an allowable stress, and the angle of twist (shown in degrees).
- Tank volume, liquid mass, bottom pressure and fill time (times of an hour or more show in hours).
- Thermal expansion of a pipe: free growth, and the stress and force if it were fully restrained, with a FAIL that shows why the line needs a loop.
- Each is checked against a hand calculation in the tests, like the first ten.

**Checks**
- The side panel lists every check with its utilisation and PASS or FAIL; click one to go to it.
- On a check line the badge now comes straight after the check, before its note.

**Sturdier**
- A line with thousands of brackets or signs inside each other could stop Datum Calc; lines are now limited (60 levels, 1,000 symbols) with a clear message, and the sheet is worked out on its own thread with plenty of room.
- A range with a tiny step (x = 0, 1e-300 .. 1) could stop Datum Calc; it now says a range holds at most 100,000 values.
- A unit power such as m^999 was quietly cut to m^127; powers of units above 60 now give a clear message.
- Results beyond about 10³⁰⁸ now say so instead of showing nothing; numbers too small to matter (below 10⁻³⁰⁰) show as 0.
- A square or cube root of a value carrying rotation (such as a torque worked out from rpm) no longer gives a units error.
- Over 100,000 random sheets (random equations, units, tables, checks, solve and plot) and 20,000 random unit strings and numbers are now worked out in the tests, in both build types, without a failure.

**Smaller**
- Degrees read 0.22° (no space). The Home tile counts the packs. Pasted pictures are limited to 4 MB.

## 0.1.4 — 2026-10-28

A Getting Started sheet, inputs that keep their units, clearer numbers, and calc packs that convert to US units.

**Getting started**
- File › Home › Getting Started (and Help › Getting Started) opens a short sheet that shows how Datum Calc works by example: inputs with units, equations and results, temperatures, a check, a function with solve and plot, and a table. Change any line and watch the rest follow.

**Results**
- An input keeps the unit you typed it in: `I = 8356 cm^4` shows 8356 cm⁴, not 83.56×10⁶ mm⁴. In a sheet set to US units, metric inputs are shown in US units (and the other way round).
- Large numbers keep only their significant figures and are grouped in threes: a Reynolds number shows as 274 900.

**Calc packs**
- In a sheet set to US units, every pack result converts (psi, ksi, in, ft, gpm, hp, °F). Moments stay in kN·m, torque in N·m and apparent and reactive power in kVA and kvar, because those units cannot be told apart from energy and power automatically.

## 0.1.3 — 2026-10-28

Ten ready calc packs, notes on every line, copying results and CSV export: the convention build.

**Calc packs** (File › New from pack)
- Rotating equipment: pump power, efficiency and motor size; NPSH available against NPSH required; pump curve against system curve (the operating point, plotted and solved); compressor polytropic head and power.
- Process: pipe pressure drop (Darcy–Weisbach with the Colebrook friction factor solved); heat exchanger duty and LMTD.
- Mechanical: thin-walled pressure vessel (hoop and longitudinal stress, required thickness); beams (simply supported and cantilever deflection and bending stress); bolt preload and tightening torque.
- Electrical: motor full-load current, apparent and reactive power, starting current.
- Each pack has its inputs at the top (highlighted), results and checks below, and its textbook or standard at the end. Each is checked against a hand calculation in Datum Calc's tests.

**Notes on a line**
- Text after `#` describes the line and shows beside it: `Q = 120 m^3/h  # rated flow`.

**Exchange**
- Right-click a region: Copy Result (`19.61 kW`, to paste into a spreadsheet or a report), Copy as LaTeX, Mark as Input, Move, Delete.
- File › Export › Inputs and results (CSV): every named value with its unit and whether it is an input or a result, and every check with PASS or FAIL and its utilisation.

## 0.1.2 — 2026-10-28

Tables, pictures and the list of every unit; a fix for gravity after a unit.

**Tables**
- Insert › Table: a small grid with a name and a unit for each column, such as a pump curve from its data sheet. Each column becomes a vector you can use below it: `H_at(q) = interp(q, Q_d, H_d)`.
- A cell without a unit takes the column's unit; a column with a formula (`rho*g*Q_d*H_d`) is worked out value by value and shown in its unit.
- Double-click a table to edit it: type from cell to cell, Enter moves down (and adds a row at the bottom), + Row and + Column grow it, Done saves it as one change you can undo.

**Pictures**
- Insert › Picture adds a sketch or a photo, such as a free-body diagram, with a caption. Paste a picture (Ctrl+V) to add it straight away. Format › Picture sets its size.

**Units, constants and functions**
- Units › All Units lists every unit by kind with its name and SI units; Math › All Functions lists the functions and constants. Search, then click one to type it into the region you are editing.

**Printing**
- Tables and pictures print with the sheet; a region that does not fit moves to the next page instead of being cut.

**Fixed**
- `1000 kg/m^3*g` read the g as grams; it is now gravity. The constants, a function's own parameters and the variable after `for` are never read as units.

## 0.1.1 — 2026-10-28

The engine arrives: every equation is worked out with its units, shown as maths, and checked.

**Units**
- About 200 units: SI with prefixes and US customary (m, mm, in, ft; kg, lb; N, kN, lbf, kip; Pa, kPa, MPa, bar, psi; J, kWh, BTU; W, kW, hp; m³/h, gpm, L/s; rpm, rad/s; and many more). Every factor is tested against the published NIST value.
- Write units after a number: `120 m^3/h`, `2.5 kN*m`, `1.2 W/(m^2*K)`, `9.81 m/s²`.
- Adding metres to seconds is an error on that line, and the wrong part of the line is underlined.
- Temperatures in °C and °F are handled correctly: 20 °C − 10 °C is a difference of 10 K, the mean of two temperatures stays a temperature, and −40 °C is −40 °F.
- Results are shown in a sensible unit (18.4 kW, 2.5 MPa, 250 mm) for the sheet's unit system (SI or US). Click ▾ after a result to show it in another unit.

**Equations**
- Every line is drawn as maths: fractions, roots, powers, subscripts (`sigma_allow` → σ with “allow” below) and Greek letters by name.
- + − × ÷, powers, brackets; sqrt, sin, cos, tan (in radians, or with `deg`), ln, log, exp, min, max, abs, round, if, sum, mean, interp, hypot.
- Your own functions: `f(x) = 3*x^2 + 2`. Ranges: `x = 0, 0.1 .. 10`.
- Constants: g, π, e, R, σ_SB (Stefan–Boltzmann). A name on the sheet takes the place of a constant.
- A name used before it is defined says so, and an error never stops the rest of the sheet.

**Checks, solve and plot**
- `check sigma <= sigma_allow` shows a green PASS or a red FAIL with both values and the utilisation (0.82 means 82 % used).
- `solve … for x in a .. b` finds the value that makes both sides equal in the range, for example the Colebrook friction factor or a pump's operating point.
- `plot H_pump(Q), H_sys(Q) for Q in 0 m^3/h .. 150 m^3/h` draws the curves, with a crosshair and a PNG button.

**The window**
- A Variables panel lists every name with its value and unit; click one to go to its line. The checks that pass and the lines with errors are counted there and in the status bar.
- The printed calc sheet shows the maths, the results and the PASS/FAIL badges.

## 0.1.0 — 2026-10-28

The first release of Datum Calc: the window, the calc sheet, saving and opening, printing, and updates from day one.

**The calc sheet**
- A page (A4 or Letter) with a title block: project, calc number, title, prepared by, checked by, date and revision, your company and logo.
- Math, text and check regions, read top to bottom like a calculation on paper. Ctrl+M adds an equation, Ctrl+T a text region; Enter ends an equation and starts the next one.
- Mark regions as inputs (highlighted), headings, notes or references.
- Move regions with Alt+↑ and Alt+↓; undo and redo every change.
- The Math tab types Greek letters, operators and functions into the region you are editing; the Units tab types units.
- This release keeps what you type; the engine that works out the results with their units arrives in 0.1.1.

**Files**
- Sheets save as `.dcalc` files. A damaged file opens with every region that can still be read.
- A recovery copy is kept while you work; recent sheets are listed on the File page.

**Printing**
- File › Print shows the pages: the title block on every page, "Page x of y" at the bottom and the references at the end. Choose Save as PDF to make a PDF.

**The family window**
- Ribbon (Home, Insert, Math, Units, Check, View, Help, and Format for the selected region), Tell me (Ctrl+K), a ? beside every tool, Options, light, dark and auto themes.
- File › Updates checks GitHub for new versions and installs them after checking their SHA-256 checksum.
