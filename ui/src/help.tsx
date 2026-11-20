// "?" help: a small button beside a tool; a click opens a short card saying what the tool is for
// and how to use it. One place for every explanation, so the whole app teaches itself.
import { For, Show } from "solid-js";
import { openDialog } from "./store";
import { Dialog } from "./ui";

type Topic = { t: string; what: string; how?: string[]; tip?: string };
export const HELP: Record<string, Topic> = {
  // Home tab
  clipboard: { t: "Clipboard", what: "Copy a region's text, or a result, to paste into another region, another sheet or a spreadsheet.",
    how: ["Select a region and press Ctrl+C; click where it goes and Ctrl+V.", "Inside a region, Ctrl+C and Ctrl+V work on the selected text as usual."] },
  regions: { t: "Regions", what: "A calc sheet is a list of regions, read top to bottom like a calculation on paper. Each region is one equation, a paragraph of text, a check, a solve, a plot, a table or a picture.",
    how: ["Math adds an equation such as Q = 120 m^3/h.", "Text adds a heading, a paragraph or a reference.", "Check adds a test such as check sigma <= sigma_allow, shown as PASS or FAIL.", "The new region goes below the selected one."] },
  arrange: { t: "Arrange", what: "Move the selected region up or down the sheet, or delete it.",
    how: ["Alt+↑ and Alt+↓ move the region; Delete removes it (Ctrl+Z brings it back).", "Duplicate (Ctrl+D) copies the region below it; Insert Above (Ctrl+Shift+M) adds an equation above it.", "Order matters: a value must be defined above the line that uses it."] },
  sheetset: { t: "Sheet", what: "The title block printed at the top of every page (project, calc number, title, by, checked, date, revision) and the sheet's unit system.",
    how: ["Title Block opens the fields; they print on every page.", "SI or US sets the units results are shown in when you have not chosen one."] },
  print: { t: "Print", what: "Print the calc sheet or save it as a PDF, with the title block, page numbers “Page x of y” and the references at the end.",
    how: ["Print / PDF (Ctrl+P) shows the pages first.", "In the print window choose your printer, or Save as PDF."] },
  // Insert tab
  insert: { t: "Insert", what: "Every kind of region: math, text, check, solve, plot, table, material, program and picture.",
    how: ["Select the region the new one goes below, then click a kind.", "Solve: solve 1/sqrt(f) = -2*log(eps/3.7 + 2.51/(Re*sqrt(f))) for f in 0.005 .. 0.1 finds the f that makes both sides equal in that range.", "Several equations together: solve Q_1 + Q_2 = Q, K_1*Q_1^2 = K_2*Q_2^2 for Q_1 = 50 m^3/h, Q_2 = 50 m^3/h — as many equations as unknowns, each from a starting guess that gives its unit.", "Vectors and matrices: K = [[2, -1], [-1, 2]] kN/m and F = [10, 0] kN, then u = lsolve(K, F) gives u in m. K*u multiplies, K[1, 2] picks an entry, and transpose, inv (or K^-1), det, dot, cross and norm work with units.", "Plot: plot H_pump(Q), H_sys(Q) for Q in 0 m^3/h .. 150 m^3/h draws both curves.", "Table: a grid with a name and a unit per column; each column becomes a vector. A column with a formula is worked out from the others.", "Data in: copy cells in a spreadsheet and paste them on the sheet (a new table) or into a table's cell (filled from there); or Insert › From CSV. A heading row such as Q (m3/h) and H [m] gives each column its name and unit; semicolons and decimal commas are read too.", "Fit: fit H(Q) = a + b*Q + c*Q^2 to Q, H finds a, b and c by least squares, with units, shows R² and defines H(Q). A model that is not linear in its parameters needs a starting guess for each: fit y(t) = A*exp(-t/tau) to t_d, y_d for A = 5 mm, tau = 1 s.", "Measured points on a plot: plot H(Q), points(Q_d, H_d) for Q in 0 .. 150 m^3/h.", "Differential equations: ode x'' = -k/m*x - c/m*x' for t in 0 .. 10 s from x = 1 cm, x' = 0 — each equation for the highest derivative, a starting value for every state, units from the starting values. The line shows the values at the end and a plot; x(t), x'(t) and x''(t) can be used below. Several equations together: ode T' = …, h' = … for t in … from T = 80 °C, h = 0 m.", "Picture: a sketch or a photo, such as a free-body diagram. Paste a picture (Ctrl+V) to add it straight away."] },
  // Math tab
  symbols: { t: "Symbols", what: "Greek letters and operators. They type into the region you are editing.",
    how: ["Or type the name: sigma becomes σ, sigma_allow becomes σ with “allow” as a subscript, sqrt(x) becomes a root.", "Powers: x^2. Multiplication: *. Division: /."] },
  functions: { t: "Functions", what: "Built-in functions: sqrt, sin, cos, tan (degrees or radians), ln, log, exp, min, max, abs, round, if, sum, mean, and for complex values re, im, abs, arg, conj and polar.",
    how: ["Click one to insert it with its brackets, then type the arguments.", "if(a > b, x, y) gives x when a > b, otherwise y.", "Complex numbers: 4j (or 4i) is 4 times the imaginary unit j. Z = (8 + 9.42j) Ω, Z = R + j*omega*L, V = 230 V∠-30° (type ∠ from Math › Complex, or /_); abs(Z) is the magnitude, arg(Z) the angle, par(Z1, Z2) two in parallel, db(H) a ratio in decibels. sin, cos, tan, sinh, cosh, tanh, exp, ln and sqrt work on complex values.", "Bode plot: plot db(H(f)), arg(H(f)) for f in 10 Hz .. 100 kHz log (log after the range gives a log frequency axis)."] },
  // Units tab
  units: { t: "Units", what: "Every number can carry a unit: 120 m^3/h, 45 m, 7.5 kW, 20 °C. Results keep their units, and adding metres to seconds is an error on that line.",
    how: ["Type the unit after the number: 2.5 kN*m, 9.81 m/s^2, 1.2 W/(m^2*K), or after brackets: (3 + 4j) Ω.", "A unit of your own: unit bbl_oil = 158.987 L, then 2 bbl_oil on the lines below, in other units (unit kbbl = 1000 bbl_oil) and in a result's unit menu. It belongs to that sheet; save the sheet as a template to keep it.", "A force times a length shows as a torque in N·m (lbf·ft in US units); choose J for work or energy.", "Any result can be shown in any unit of the same kind: its unit menu › Other unit…, then type it (kN*mm, lbf*in, W/(m^2*K)).", "ω = 2*pi*f shows in rad/s; a frequency in Hz shown in rpm counts revolutions (50 Hz = 3000 rpm).", "SI or US sets the default units of results; a result's own unit can be chosen next to it."] },
  // Check tab
  checks: { t: "Checks", what: "A check compares two values, such as a stress against its allowable, and shows PASS in green or FAIL in red, with the utilisation ratio.",
    how: ["Type check sigma <= sigma_allow (or <, >, >=).", "The ratio is the left side over the right side: 0.82 means 82 % used."] },
  // View tab
  zoom: { t: "Zoom", what: "How large the page shows on the screen. Printing is not affected." },
  figures: { t: "Significant figures", what: "How many figures every result on this sheet shows: 3 for a quick estimate, 4 as usual, 5 or 6 when results are compared closely. The engine always works at full precision; only the display changes. It is saved with the sheet." },
  show: { t: "Show", what: "Variables lists every name the sheet defines and every check. Numbers shows each equation a second time with the values put in (P = ρgQH/η = 998 kg/m³ · 9.807 m/s² · … = 19.57 kW), as on a hand calculation; it is saved with the sheet and printed. Polar shows complex values as magnitude and angle (5∠53.13° Ω) instead of (3 + 4j) Ω. Fractions shows results that are fractions as fractions (1/3, not 0.3333) and whole numbers in full; a result in a unit reads best with that unit chosen (7/12 m)." },
  program: { t: "Program", what: "A few lines that decide and repeat: if, else if, else, for, while and break, each block closed by end. Units work on every line. Run once, every name it sets is defined below; with a first line such as f(Re, k) = it is a function that returns a value.",
    how: ["One statement per line; Enter starts a new line and Tab indents. Close each if, for and while with end.", "for i in 1 .. 10, for x in 0, 0.5 m .. 3 m, or for F_i in F (each value of a vector).", "A function: f_D(Re, k) = on the first line, then return 64/Re for laminar flow, or iterate the Colebrook equation and return f.", "Loops stop after 100,000 rounds, so a loop that never ends says so instead of hanging."] },
  material: { t: "Material", what: "The properties of a material from the library (from Datum Atlas, about 250 materials): density, moduli, strengths, elongation, temperatures, conductivity, expansion, specific heat, resistivity. Each becomes a name with its unit for the lines below.",
    how: ["Insert › Material, search (S355, 6061, nylon, oak), click one and give it a short name such as steel: its properties become ρ_steel, E_steel, σ_y,steel, λ_steel …", "Values: typical (the middle of each range), or the low or high end.", "The values are kept in the sheet, so it reads the same when the library is updated. The sheet shows the source of each.", "These are typical values for generic grades: for design, check the standard or the supplier's datasheet."] },
  props: { t: "Properties", what: "Water, steam and gases as functions with units. Water and steam follow IAPWS-IF97 (0 °C to 800 °C up to 1000 bar, to 2000 °C up to 500 bar), with viscosity (IAPWS 2008), conductivity (IAPWS 2011) and surface tension (IAPWS 2014). Gases are ideal gases: heat capacity from NASA polynomials (200 K to 3000 K), viscosity and conductivity fitted to reference correlations within 0.2 %.",
    how: ["Give any two of temperature, pressure, enthalpy, entropy or vapour fraction, in either order; the units tell them apart: water_h(400 °C, 40 bar), water_h(0.1 bar, s_1) after an expansion, water_x(0.1 bar, h_2) for the dryness.", "Properties: water_rho, _v, _h, _u, _s, _cp, _cv, _w (speed of sound), _T, _p, _x, _mu, _nu, _k, _Pr; water_Tsat(p), water_psat(T), water_sigma(T).", "Inside the saturation dome (wet steam) density, enthalpy and entropy are mixed by the vapour fraction; cp, viscosity and conductivity are not defined there. Outside it, water_x gives the thermodynamic quality: below 0 for liquid, above 1 for superheated steam.", "Gases: gas_cp(air, T), gas_cv, gas_gamma, gas_h (above 25 °C), gas_s(air, T, p), gas_rho(air, T, p), gas_mu, gas_nu(air, T, p), gas_k, gas_Pr, gas_M, gas_R, for air, N2, O2, Ar, CO2, H2, He and CH4. Density is the ideal gas law: close at a few bar, less so near condensation.", "A range of temperatures gives a vector of properties: T = 20 °C, 40 °C .. 100 °C, then water_rho(T, 1 bar).", "Temperatures are states, not differences: 300 K and 27 °C are the same state; a difference such as T_2 − T_1 is not a temperature to look up."] },
  algebra: { t: "Algebra", what: "Exact algebra on expressions: names that are not defined are symbols (x, y), defined plain numbers are put in, and the result is an expression, not a number.",
    how: ["simplify((x^2 - 1)/(x - 1)) gives x + 1: like terms collected, fractions cancelled.", "expand((x + 1)^2) gives x² + 2x + 1.", "factor(2*x^2 - x - 1) gives (2x + 1)(x − 1); factor(6*x^2*y + 9*x*y) gives 3xy(2x + 3).", "y = expand((x + 1)^3) keeps the expression in y, so factor(y) works on a later line.", "It works on numbers, names, + − × ÷ and whole powers; not on units, functions such as sin, or decimals that are not fractions."] },
  vars: { t: "Variables", what: "Every name the sheet defines, with its value and unit, in the order they are defined. The checks and errors are counted at the bottom.",
    how: ["Click a name to go to the line that defines it.", "View › Variables shows or hides the panel."] },
  results: { t: "Results", what: "Each equation shows its answer with a unit chosen for it: kW for power, MPa for stress, m³/h for flow. The unit system (SI or US) sets the defaults.",
    how: ["Click ▾ after a result to show it in another unit (hp instead of kW, psi instead of MPa).", "Automatic goes back to the unit Datum Calc chooses."] },
  theme: { t: "Theme", what: "Light, dark, or Auto (follows Windows). Printed sheets are always black on white." },
  // Pages and panels
  packs: { t: "Calc packs", what: "Ready, referenced calc sheets, by field: rotating equipment (pumps, compressors, shafts, belt drives), process (pipe pressure drop, heat exchangers, tanks, insulation), mechanical (vessels, beams, columns, bolts, flanges, welds, springs) and electrical (motors, cables, three-phase loads, AC circuits, filters).",
    how: ["File › New › pick a pack.", "Change the highlighted inputs at the top to your case; everything below follows.", "Each pack is checked against a hand calculation in Datum Calc's own tests."] },
  page: { t: "The calc sheet", what: "The page as it will print. Regions are worked out from top to bottom.",
    how: ["Click a region to select it; double-click (or Enter) to edit its text; Esc or a click outside finishes.", "Insert adds a region below the selected one."] },
  export: { t: "Export", what: "The calc sheet as a Word document or as a web page, as well as PDF (Print) and CSV. Regions hidden in print are left out.",
    how: ["File › Export › Word writes a .docx: the title block, headings and text, each equation as a Word equation you can edit, its result, checks with PASS or FAIL, tables, plots and pictures.", "File › Export › Web page writes one .html file that any browser shows without the internet: the maths as MathML, plots and pictures inside the file.", "Right-click a region › Hide in Print (or Home › Hide in Print) keeps it out of print and exports; it is still worked out and shown faint on the screen.", "The arrow beside a heading folds its section away on the screen; it still prints."] },
  templates: { t: "Templates", what: "Sheets of your own to start new ones from: your company's standard layout, a calculation you repeat, a title block filled in. They are kept with Datum Calc's settings on this computer.",
    how: ["File › New › Save as template… saves the open sheet under a name; its tile then starts a new sheet with its regions and title block, dated today.", "Delete on a tile removes that template; sheets made from it are not affected.", "Title block › Use for new sheets makes the company, client, people and logo the start of every new sheet, pack and template."] },
  titleblock: { t: "Title block", what: "The block at the top of every printed page: project, calc number, title, prepared by, checked by, date and revision, plus your company and its logo.",
    how: ["Defaults for new sheets (company, your name, logo) are in File › Options."] },
};

/** The small "?" beside a tool. */
export const Help = (p: { k: string }) => <button class="help" title={`What is ${HELP[p.k]?.t ?? "this"}?`} onClick={e => { e.stopPropagation(); helpCard(p.k); }}>?</button>;

export const helpCard = (k: string) => openDialog(() => {
  const h = HELP[k];
  return (
    <Dialog title={h?.t ?? "Help"}>
      <div class="helpcard"><p>{h?.what}</p>
        <Show when={h?.how}><h4>How to use it</h4><ul><For each={h!.how}>{x => <li>{x}</li>}</For></ul></Show>
        <Show when={h?.tip}><p class="muted">{h!.tip}</p></Show></div>
    </Dialog>
  );
});
