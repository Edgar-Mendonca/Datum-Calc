# Changelog

## 0.2.7 — 2026-11-20

Data in and curve fitting.

**Data in**
- Copy cells in a spreadsheet and paste them on the sheet: they become a table. Paste them into a table's cell and they fill it from there, adding rows and columns as needed.
- Insert › From CSV reads a CSV or text file into a table. Tabs, semicolons or commas (whichever most lines share, with a title line above the table left out), decimal commas, thousands separators and quoted cells, line breaks inside quotes included, are read; a UTF-16 file is read too; a heading such as Q (m3/h) or H [m] gives each column its name and unit, and a second row of units is read too. What could not be read (an unknown unit, rows beyond 2,000) is said.

**Curve fitting**
- fit H(Q) = a + b*Q + c*Q^2 to Q, H finds a, b and c by least squares from two columns of data, each with its units (m, m/(m³/h), m/(m³/h)²), shows R², and defines H(Q) for the lines below.
- Any model that is a sum of terms, each proportional to one parameter, is fitted at once: polynomials, a + b/x, a*x + b*ln(x). Other models (A*exp(-t/tau), k*x^n) are fitted from a starting guess for each parameter, which also gives its unit.
- The parameters are the names the sheet does not define, or those listed after for; names the fit holds from the sheet are said. A fitted function keeps its own fitted values, even when a later fit uses the same names.
- It says when the data have an empty cell, when a parameter does not change the model, when a fit from guesses does not settle, and when R² is not defined (all y the same). Temperatures in °C stay absolute: fitting T in °C gives the intercept in °C and the slope in K.
- A vector may hold temperatures: [20, 30, 40] °C, as a range or a table column can.
- points(xs, ys) draws measured values on a plot beside the curves: plot H(Q), points(Q_d, H_d) for Q in 0 .. 150 m^3/h.
- New calc pack (29 in all): a pump curve fitted to test data, the system curve and the operating point, checked against an independent least-squares and root calculation.

**Checks**
- 92 engine tests (fits against independent results, linear and nonlinear; reading spreadsheet and CSV text; the new pack) and 107 UI tests (a fit and its plot, Insert › From CSV); random pasted text and random fit lines never panic. Every earlier pack prints as before.

## 0.2.6 — 2026-11-20

Water, steam and gas properties.

**Water and steam**
- Properties as functions with units, from any two of temperature, pressure, enthalpy, entropy or vapour fraction, in either order: water_h(400 °C, 40 bar), water_h(0.1 bar, s_1) after an expansion, water_x(0.1 bar, h_2) for the dryness, water_T(p, h).
- Density, specific volume, enthalpy, internal energy, entropy, cp, cv, speed of sound, viscosity, kinematic viscosity, thermal conductivity and Prandtl number; water_Tsat(p), water_psat(T) and water_sigma(T) for surface tension.
- IAPWS-IF97 from 0 °C to 800 °C up to 1000 bar and to 2000 °C up to 500 bar, near the critical point included; viscosity IAPWS 2008, conductivity IAPWS 2011, surface tension IAPWS 2014. Checked against the verification tables of the IAPWS releases.
- Wet steam (inside the saturation dome) mixes density, enthalpy and entropy by the vapour fraction; it says so when a property such as cp is not defined there.

**Gases**
- Air, nitrogen, oxygen, argon, carbon dioxide, hydrogen, helium and methane as ideal gases: gas_cp(air, T), gas_cv, gas_gamma, gas_h, gas_s(air, T, p), gas_rho(air, T, p), gas_mu, gas_nu, gas_k, gas_Pr, gas_M, gas_R.
- Heat capacity within about 0.4 % from 200 K to 3000 K (NASA Glenn polynomials); viscosity and conductivity within 0.2 % of reference correlations over each gas's range.

**Also**
- A range of temperatures or pressures gives a vector of properties, for a table or a plot.
- Math › Properties keys, and a help card with the ranges and accuracy.
- New calc pack (28 in all): a steam turbine Rankine cycle, checked against an independent IF97 calculation.

**Checks**
- 86 engine tests (the IAPWS verification tables, states within millikelvins of the critical point, viscosity check values, properties on a sheet, what it says out of range, the new pack) and 104 UI tests; random states and random sheets with property calls never panic. Every earlier pack prints as before.

## 0.2.5 — 2026-11-19

Small programs in a region.

**Programs**
- New region: Insert › Program. A few lines that decide and repeat, with units on every line: if, else if, else, for, while and break, each block closed by end.
- Run once, every name the program sets is defined for the lines below and shown as its result (total = 60 kN, n = 10).
- With a first line such as f_D(Re, k) = it is a function that returns a value: return 64/Re for laminar flow, or iterate the Colebrook equation and return f. Use it on any line, in a plot or in solve.
- for i in 1 .. 10, for x in 0, 0.5 m .. 3 m, or for F_i in F (each value of a vector).
- Drawn as maths, line by line, with the keywords in bold. Enter starts a new line; Tab indents the selected lines, Shift+Tab takes the indent away, and Ctrl+Z undoes either.
- It says what is wrong and on which line: a block without end, an else without if, a loop that never ends (stopped after 100,000 rounds), units that do not match. A plot or solve that calls a slow program function stops after 1,000,000 rounds on that line and says so.
- A function's working names are its own: names and expressions on the sheet with the same spelling are left as they were. A program that fails part way changes nothing on the sheet.
- New calc pack (27 in all): pipe flow in any regime, with the friction factor as a program and checked against a hand calculation.

**Checks**
- 81 engine tests (functions, loops, branches, break, recursion, errors and where they point, the limits on a plot or solve, the new pack), 103 UI tests (a program draws, Insert › Program, Tab and Shift+Tab); the random-sheet test writes programs too. Every earlier pack prints as before.

## 0.2.4 — 2026-11-18

Vectors and matrices with units.

**Matrices**
- Write a vector as [10, 0] kN and a matrix as [[2, -1], [-1, 2]] kN/m, with one unit for all its entries. It is shown in brackets, as on paper.
- K*u multiplies matrices, K + M and 2*K work entry by entry (two vectors multiply entry by entry, as for ranges; use dot for the dot product), K^2 and K^-1 are powers and the inverse, and K[1, 2] picks an entry (counting from 1).
- New functions: lsolve(A, b) for a linear system, inv, det, transpose, identity(n), dot, cross, norm, rows and cols.
- Units follow the algebra: lsolve of a stiffness in kN/m and loads in kN gives displacements in m; the determinant of a 2 × 2 in kN/m is in N²/m².
- It says what is wrong: different shapes, rows of different lengths, an index out of range, a singular matrix, entries in different units, or absolute temperatures (°C) in a matrix.
- Limits keep it quick: a matrix holds up to 10,000 values, and one factorisation serves lsolve, inv and det.

**Fixes**
- Several equations solved together (0.2.3) now cope with unknowns of very different sizes, such as a modulus in GPa beside a strain, and guesses far from the answer.

**Checks**
- 78 engine tests (products, inverse, determinant, linear systems with units, indexing, singular and badly scaled matrices, what it says when it cannot), 100 UI tests (a matrix draws in brackets); the random-sheet test writes matrices too. Every earlier pack prints as before.

## 0.2.3 — 2026-11-18

Several equations solved together.

**Systems of equations**
- solve now takes several equations, separated by commas, and an unknown for each, each from a starting guess: solve Q_1 + Q_2 = Q, K_1*Q_1^2 = K_2*Q_2^2 for Q_1 = 50 m^3/h, Q_2 = 50 m^3/h. Each unknown takes the unit of its guess, and every unknown is defined for the lines below and listed in the Variables panel.
- One equation can be solved from a guess too (solve exp(t) = 5 for t = 0), as well as in a range as before.
- When the equations cannot be solved it says why: more unknowns than equations, equations that do not fix every unknown (2x + 2y = 2 with x + y = 1), units that do not match, or no solution near the guesses.
- New calc pack (26 in all): parallel pipes, the flow split between two pipes that share a pressure drop, solved together and checked against a hand calculation.

**Checks**
- 76 engine tests (systems with and without units, the beam reactions, a resonance in mixed units, what it says when it cannot, the new pack), 99 UI tests; the random-sheet test writes systems too. Every earlier pack prints as before.

## 0.2.2 — 2026-11-11

Five more calc packs (25 in all), each checked against a hand calculation.

**New calc packs**
- Bolted flange: the operating and gasket-seating bolt loads and the bolt area needed against the bolt area provided (ASME Section VIII Division 1, Appendix 2).
- Tank heating time with heat losses: the exact time for a heater to bring a well-mixed tank to temperature while it loses heat, against the time allowed.
- Three-phase load: line current, apparent and reactive power, the capacitor bank for power-factor correction (kvar and µF per phase) and the current after correction against the cable rating.
- Helical compression spring: rate, deflection and shear stress with the Bergsträsser factor, music wire's strength from its diameter, against the static allowable.
- Flat belt drive: wrap angle, belt speed, tight- and slack-side tensions, driven speed and belt length, against the rated tension.

**Equations with their numbers**
- An angle is put in as radians: R = e^(μθ) = e^(0.3·2.84 rad), not e^(0.3·162.7°), which read as 48.8 in the exponent.

**Checks**
- 74 engine tests (each new pack against its hand calculation), 99 UI tests. The earlier packs print exactly as before. An independent engineering check of the five packs agreed with every equation, input and result; it corrected two section numbers in the references and asked for wording that stays true when inputs change.

## 0.2.1 — 2026-11-11

Materials: the properties of about 250 materials, from Datum Atlas, as names with units on the sheet.

**Material regions**
- Insert › Material opens a library of 252 materials (metals, polymers, elastomers, ceramics and glasses, composites, foams and natural materials) with a search: S355, 6061, nylon, oak, alumina.
- Pick one and give it a short name such as steel: its properties become names with units for the lines below — ρ_steel, E_steel, G_steel, ν_steel, σ_y,steel, σ_ts,steel, σ_c,steel, σ_e,steel, ε_f,steel, T_max,steel, λ_steel, α_steel, c_p,steel, ρ_e,steel and the others the material has.
- Choose typical values (the middle of each range), or the low or the high end.
- The region shows a small datasheet on the sheet and in print: each property with its value, unit, range and source.
- The values are kept in the sheet when the material is picked, so a calculation reads the same when the library is updated later.
- The library comes from Datum Atlas (MIT), where each value names its source. Atlas marks its values as typical ranges for generic grades, still being referenced, and the datasheet says so: for design, check the standard or the supplier's datasheet.
- New units: µΩ and nΩ.

**Checks**
- 69 engine tests (new: every one of the 252 materials worked out in SI and US units; S355's values and units; low and high ends; a sheet keeps its values; what a material region says when it cannot; damaged material data), 99 UI tests (the datasheet, the picker on the real library, a search, a pick and a rename). Every pack prints as before. The Windows install check passes.
- An independent review before release found that renaming a material region that named only its material dropped the material, that two materials could get the same suggested name (S355 and 304 are both "steel") and the second replaced the first without a word, and that elongation lost its % on US sheets. The suggested name is now unique (steel, steel2), a reused name gets a warning, renaming keeps the material, and % stays.

## 0.2.0 — 2026-11-11

Exact maths: fractions that stay fractions, and algebra on expressions.

**Algebra**
- simplify, expand and factor work on expressions. Names that are not defined are symbols (x, y, π); plain numbers defined above are put in.
- expand((x + 1)^2) gives x² + 2x + 1; simplify((x^2 - 1)/(x - 1)) gives x + 1; factor(2*x^2 - x - 1) gives (2x + 1)(x − 1); factor(6*x^2*y + 9*x*y) gives 3xy(2x + 3); factor(x^3 - 6*x^2 + 11*x - 6) gives (x − 1)(x − 2)(x − 3).
- y = expand((x + 1)^3) keeps the expression in y, so factor(y) works on a later line.
- The coefficients are exact fractions, never rounded. Exact maths works on numbers, names, + − × ÷ and whole powers; a line with units, functions such as sin, or a decimal that is not a fraction says so.
- Math › Algebra has keys for the three; Help › What Is This Tool? › Algebra explains them.

**Fractions**
- View › Fractions shows results that are fractions as fractions (1/3, not 0.3333; 0.1 + 0.2 is 3/10) and whole numbers in full. It is saved with the sheet and off by default, so engineering sheets keep their decimals. π, √2 and other numbers that are not fractions stay decimals.

**Checks**
- 62 engine tests (new: algebra worked by hand, a thousand random polynomials whose factors multiply back and whose fractions cancel, fractions shown exactly, the slowest inputs in under a second), 93 UI tests. Every pack prints as before. 36,000 more random sheets with the new functions. The Windows install check passes.
- An independent review before release found that a short line of nested powers could fill memory and close the app, that a very large power could wrap round to x⁰, and that some answers read ambiguously as text (x + 1/x − 1); powers above 1000 are now refused at once, and text is bracketed where it needs to be: (x + 1)/(x − 1), 1/(2x), (2/3) m. A name given a value after an expression drops the expression, and names that are defined further down, have an error, or are functions are refused instead of taken for symbols.

## 0.1.12 — 2026-11-11

Every release is now installed and used on a clean Windows machine before it is published.

**Windows install check**
- A new check runs on GitHub's Windows machines for every change and before every release. It installs the previous release, then the new one over it (as the in-app updater does), and checks the version, the publisher (Datum) and that .dcalc files open with Datum Calc.
- It then uses the installed app as a person would: it opens a sheet double-clicked in Explorer, checks its maths and results, makes an edit, sees Save changes? before New, opens the print preview and keeps a recovery copy. Then it ends the app as a crash would, and checks that the unsaved work is offered back, that Discard removes it and that a calc pack opens and works out. Last, it uninstalls and checks nothing is left.
- A release is not published unless all of this passes.

**Other**
- The README's family table shows its app icons again.
- Comments describe features only; README and NOTICE end with a note on trademarks.
- The window is now made by the app from its settings, so the install check can reach it; nothing changes for users.

## 0.1.11 — 2026-11-01

Complex numbers and units, completed: AC circuits in the frequency domain, Bode plots, and any result in any unit of its kind.

**Frequency-domain analysis**
- Phasors as magnitude and angle: V = 230 V∠-30° (∠ is on Math › Complex; or type /_). A bare number after ∠ is in degrees (230∠-30). ∠ binds before × but not inside a power: 2*10∠90° is 20j, 10^2∠30° is 100∠30°.
- par(Z1, Z2, …) puts impedances in parallel; db(H) is a ratio in decibels, 20·log₁₀|H|.
- sin, cos, tan, sinh, cosh and tanh work on complex values (transmission lines, cosh(γl)), as exp, ln and sqrt already did.
- Tables hold complex values: a formula column such as R + j*2*pi*f_t*L gives an impedance per frequency, and a cell can be typed as 3 + 4j with the column's unit.
- Log frequency axes: plot db(H(f)), arg(H(f)) for f in 10 Hz .. 100 kHz log draws a Bode plot, with points spaced evenly in decades and the decades labelled.
- New calc pack (20 in all): RC low-pass filter, with its cut-off frequency, gain and phase at a frequency to suppress, and both Bode plots; checked against a hand calculation.

**Units: one kind, several meanings**
- ω = 2*pi*f (and 2*pi/T) shows in rad/s, and ω/(2*pi) in Hz again; 1/(2*pi)*sqrt(k/m) is a natural frequency in Hz. Nothing worked out from ω, such as ωL/R or 1/(1 + jωτ), is mistaken for an angle. A plot over ω, or of ω and f together, reads in the axis' own unit.
- Hz counts cycles and rpm revolutions: 50 Hz shown in rpm is now 3000 rpm (it read 477.5), and 314.2 rad/s shown in Hz is 50 Hz. rad/s is in the unit menu of every frequency.
- A kind with no usual unit of its own is written in named units instead of base units: W/m, m·K/W, J·s, N·s, V/m, W/K (it was kg·m/s³ or m²·kg/s). Every such label can be typed back as a unit, including 1/(m·s) and powers written as m⁴. Heat flux shows in W/m² (BTU/(h·ft²) in US units), mass flux in kg/(m²·s), a moment of inertia in kg·m².
- Any result can be shown in any unit of its kind: its unit menu › Other unit…, then type it (kN*mm, lbf*in, W/(m^2*K)). A unit that does not fit says so on the line.

**Checks**
- 57 engine tests (new: angular frequency and named units, complex AC analysis with ∠, par, db, complex trigonometry and complex tables, the RC filter with its log plots; every automatic label for 624 kinds typed back), 89 UI tests (Other unit…, Bode plots on a log axis, the Complex keys). Every earlier pack prints exactly as in 0.1.10. 18,000 more random sheets with the new syntax. An independent review of the changes found seven issues before release (among them 1/(2π)·√(k/m) shown in rad/s, and a plot over ω scaled by 2π); all fixed, with tests.

## 0.1.10 — 2026-10-30

Unsaved work is safer: New, Open and an update ask first. Fixes found by an independent review of 0.1.8 and 0.1.9.

**Unsaved work**
- New, Open, a calc pack, a recent sheet, Getting Started and installing an update now ask “Save changes?” when the sheet has unsaved changes. Before, they replaced it without asking; only closing the window asked.
- An update writes a recovery copy of unsaved work before the window closes, so the new version offers it back (not when you chose Don't save).
- An update installs only once its checksum has been compared with SHA256SUMS.txt.
- The recovery box names the copy it offers, and Recover or Discard acts on that copy only. With two copies and two windows starting together, Discard in one window could delete a copy the other had not shown.
- A window that cannot lock its recovery copy tries another name, and a lock made in the last minute is never cleaned away, so a running window's copy never looks abandoned.
- A sheet saved under a name typed without .dcalc gets the extension, so Explorer opens it with Datum Calc; it never replaces an existing file without asking.

**Results**
- A torque typed with its unit (50 N*m) keeps reading in N·m in later lines (M2*2 = 100 N·m, not 100 J); 50 J stays an energy. A torque times an angle is work, in J.
- A unit after brackets gives a plain or complex number its unit, (8 + 6j) Ω; on a value that already has units, such as (F/A) MPa, it is an error that points to the unit menu, as before 0.1.8.
- A complex number written out (Z = (8 + 6j) Ω) is an input, so its value is not repeated; 0j is real.
- Getting Started shows torque and complex numbers.

**Checks**
- 54 engine tests and a shell test, 86 UI tests (Save changes? before New, a pack and a recent sheet; Cancel and Don't save). Recovery tried again in the real window with two copies and two windows. 12,000 more random sheets.

## 0.1.9 — 2026-10-30

Safer recovery when several sheets are open, two more calc packs, and smaller fixes.

**Recovery**
- Each Datum Calc window keeps its own recovery copy. Two sheets double-clicked in Explorer run as two windows; before, they shared one copy, and closing or saving one deleted the other's. A copy is offered again only when the window that kept it is no longer running (closed without saving, or crashed).
- Discard in the “Recover unsaved work?” box now deletes the copy; closing the box with × keeps it for next time. A copy kept by 0.1.8 is still offered.

**More calc packs** (19 in all)
- Heat loss from an insulated pipe: radial conduction through the insulation and the outer surface coefficient in series, the heat lost per metre and along the line, and the surface temperature for personnel protection.
- Fillet weld in shear: the simplified method for a pair of fillet welds, with the minimum throat and length.
- Each is checked against a hand calculation in the tests.

**Smaller fixes**
- j (or i) is drawn upright in equations when it is the imaginary unit, as in the results; a name of the sheet called j stays italic.
- A check on absolute temperatures (29.6 °C ≤ 60 °C) shows no utilisation; a ratio of kelvins meant nothing.

**Checks**
- 54 engine tests, 82 UI tests (Discard and × in the recovery box); 18,000 more random sheets. Recovery was tried in the real window: a crashed window's copy offered and recovered, a running window's copy never offered, Discard, a copy from 0.1.8, and two windows at once.

## 0.1.8 — 2026-10-30

What visitors asked for: complex numbers for AC circuits, and torque in N·m.

**Complex numbers**
- Type 4j (or 4i) for an imaginary number, and j (or i) on its own for the imaginary unit: Z = R + j*omega*L. A name on the sheet called j or i takes their place; 4j stays imaginary.
- A unit after brackets applies to all of them: Z = (8 + 9.42j) Ω.
- +, −, ×, ÷ and powers work on complex values, as do sqrt, exp, ln, abs (the magnitude), sum and mean. New functions: re, im, arg (the angle), conj and polar(r, θ).
- Results show as (3 + 4j) Ω, or as magnitude and angle (5∠53.13° Ω) with View › Polar, saved with the sheet. Equations with their numbers show the complex values too.
- Checks, comparisons, solve, plots, ranges and tables work on real values and say so: compare abs(I), plot abs(Z) or arg(Z).
- New calc pack (17 in all): AC circuit, an R–L load with a capacitor in parallel for power-factor correction, with its impedance, current phasor, complex power and power factor checked against a hand calculation.

**Torque**
- A force times a length shows in N·m (lbf·ft in US units), and so does a power over a speed: 120 N × 0.35 m = 42 N·m, not 42 J. A power times a time still shows in J.
- N·m, kN·m, N·mm, MN·m, lbf·ft, lbf·in and kip·ft are in the unit menu of every torque and energy result.

**Checks**
- 52 engine tests (three new ones for complex numbers, torque and the AC pack, with the hand values first), 80 UI tests. The random-sheet test now also writes complex numbers and units after brackets; 24,000 more random sheets came through without a fault.

## 0.1.7 — 2026-10-29

- Windows now lists the publisher as Datum.

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
