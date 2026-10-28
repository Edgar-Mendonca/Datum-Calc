# Changelog

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
