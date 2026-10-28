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
    how: ["Alt+↑ and Alt+↓ move the region; Delete removes it (Ctrl+Z brings it back).", "Order matters: a value must be defined above the line that uses it."] },
  sheetset: { t: "Sheet", what: "The title block printed at the top of every page (project, calc number, title, by, checked, date, revision) and the sheet's unit system.",
    how: ["Title Block opens the fields; they print on every page.", "SI or US sets the units results are shown in when you have not chosen one."] },
  print: { t: "Print", what: "Print the calc sheet or save it as a PDF, with the title block, page numbers “Page x of y” and the references at the end.",
    how: ["Print / PDF (Ctrl+P) shows the pages first.", "In the print window choose your printer, or Save as PDF."] },
  // Insert tab
  insert: { t: "Insert", what: "Every kind of region. Math, text and check regions work now; solve, plot, table and picture regions arrive in the next releases.",
    how: ["Select the region the new one goes below, then click a kind."] },
  // Math tab
  symbols: { t: "Symbols", what: "Greek letters and operators. They type into the region you are editing.",
    how: ["Or type the name: sigma becomes σ, sigma_allow becomes σ with “allow” as a subscript, sqrt(x) becomes a root.", "Powers: x^2. Multiplication: *. Division: /."] },
  functions: { t: "Functions", what: "Built-in functions: sqrt, sin, cos, tan (degrees or radians), ln, log, exp, min, max, abs, round, if, sum, mean.",
    how: ["Click one to insert it with its brackets, then type the arguments.", "if(a > b, x, y) gives x when a > b, otherwise y."] },
  // Units tab
  units: { t: "Units", what: "Every number can carry a unit: 120 m^3/h, 45 m, 7.5 kW, 20 °C. Results keep their units, and adding metres to seconds is an error on that line.",
    how: ["Type the unit after the number: 2.5 kN*m, 9.81 m/s^2, 1.2 W/(m^2*K).", "SI or US sets the default units of results; a result's own unit can be chosen next to it."] },
  // Check tab
  checks: { t: "Checks", what: "A check compares two values, such as a stress against its allowable, and shows PASS in green or FAIL in red, with the utilisation ratio.",
    how: ["Type check sigma <= sigma_allow (or <, >, >=).", "The ratio is the left side over the right side: 0.82 means 82 % used."] },
  // View tab
  zoom: { t: "Zoom", what: "How large the page shows on the screen. Printing is not affected." },
  show: { t: "Show", what: "Panels beside the page." },
  theme: { t: "Theme", what: "Light, dark, or Auto (follows Windows). Printed sheets are always black on white." },
  // Pages and panels
  page: { t: "The calc sheet", what: "The page as it will print. Regions are worked out from top to bottom.",
    how: ["Click a region to select it; double-click (or Enter) to edit its text; Esc or a click outside finishes.", "Insert adds a region below the selected one."] },
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
