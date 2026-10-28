// Dialogs: title block, getting started, glossary, shortcuts, save before closing.
import { For, Show, createSignal, createResource } from "solid-js";
import { Lists, Out, TitleBlock } from "./api";
import { typeText } from "./cmds";
import { act, call, closeDialog, openDialog, S, opts } from "./store";
import { Dialog } from "./ui";

const FIELDS: [keyof TitleBlock, string][] = [["project", "Project"], ["number", "Calc number"], ["title", "Title"], ["by", "Prepared by"], ["checked", "Checked by"], ["date", "Date"], ["rev", "Revision"], ["company", "Company"], ["client", "Client"]];
export const titleBlock = () => openDialog(() => {
  const [t, setT] = createSignal<TitleBlock>({ ...S().title });
  const ok = () => act({ Batch: FIELDS.filter(([k]) => t()[k] !== S().title[k]).map(([k]) => ({ Title: { field: k, value: t()[k] } })).concat(t().logo !== S().title.logo ? [{ Title: { field: "logo", value: t().logo } }] : []) });
  return (
    <Dialog title="Title block" help="titleblock" ok={ok}>
      <div class="form">
        <For each={FIELDS}>{([k, l]) => <><label>{l}</label><input value={t()[k]} onInput={e => setT({ ...t(), [k]: e.currentTarget.value })} /></>}</For>
        <label>Logo</label><div class="inline">{t().logo && <img src={t().logo} class="logoprev" />}
          <button class="btn sm" onClick={async () => { const l = await call<string | null>("pick_image"); if (l) setT({ ...t(), logo: l }); }}>Choose picture…</button>
          <Show when={t().logo}><button class="btn sm" onClick={() => setT({ ...t(), logo: "" })}>Remove</button></Show>
          <Show when={!t().logo && opts().logo}><button class="btn sm" onClick={() => setT({ ...t(), logo: opts().logo })}>Use my company logo</button></Show></div>
      </div>
    </Dialog>
  );
});

export const gettingStarted = () => openDialog(() => (
  <Dialog title="Getting started" wide>
    <div class="gloss">
      <h4>A calc sheet in three steps</h4>
      <dl><dt>1. Write the equations</dt><dd>Insert a math region (Ctrl+M) and type an equation with its units, such as <code>Q = 120 m^3/h</code>. Each line can use the names defined above it: <code>P = rho*g*Q*H/eta</code>.</dd>
        <dt>2. Check the results</dt><dd>Insert a check such as <code>check sigma &lt;= sigma_allow</code>. It shows PASS or FAIL and how much of the limit is used.</dd>
        <dt>3. Print the calc sheet</dt><dd>Fill in the title block (Home › Title Block) and press Ctrl+P. Choose Save as PDF to make a PDF to hand in.</dd></dl>
      <h4>Typing maths</h4>
      <dl><dt>Names</dt><dd>Greek letters by name (<code>sigma</code>, <code>rho</code>, <code>eta</code>); a subscript after an underscore (<code>sigma_allow</code>).</dd>
        <dt>Operators</dt><dd><code>+ - * / ^</code> and brackets; <code>sqrt(x)</code>, <code>sin(30 deg)</code>, <code>ln</code>, <code>exp</code>, <code>min</code>, <code>max</code>, <code>abs</code>, <code>if(a &gt; b, x, y)</code>.</dd>
        <dt>Units</dt><dd>Put the unit after the number: <code>2.5 kN*m</code>, <code>9.81 m/s^2</code>, <code>20 °C</code>. Results keep their units; mixing metres and seconds is an error shown on that line.</dd></dl>
      <h4>Finding things</h4>
      <dl><dt>Tell me (Ctrl+K)</dt><dd>Type what you want to do, such as “print”, “check”, “units”.</dd><dt>The ? beside each tool</dt><dd>Says what the tool is for and how to use it.</dd></dl>
    </div>
  </Dialog>
));

const GLOSSARY: [string, [string, string][]][] = [
  ["Calc sheets", [["Region", "One block of the sheet: an equation, text, a check, a solve, a plot, a table or a picture. Regions are worked out from top to bottom."],
    ["Title block", "The box at the top of each printed page with the project, calc number, title, who prepared and checked it, the date and the revision."],
    ["Revision", "A letter or number that changes each time the calculation is issued again (A, B, C or 0, 1, 2)."],
    ["Reference", "The textbook, standard or paper an equation comes from, printed at the end of the sheet."]]],
  ["Units", [["Dimension", "What kind of quantity a value is: length, mass, time, temperature, current, amount of substance, luminous intensity, or a combination such as force (mass × length / time²)."],
    ["Absolute and difference temperature", "20 °C is a temperature (293.15 K); a rise of 20 °C is a difference of 20 K. Datum Calc keeps them apart so conversions are right."],
    ["SI and US", "The two unit systems results can be shown in: SI (m, kg, s, N, Pa, W) or US customary (ft, lb, psi, hp, °F)."]]],
  ["Checks", [["Utilisation", "The value checked divided by its limit. 0.82 means 82 % of the limit is used; above 1.00 the check fails."],
    ["PASS / FAIL", "Whether the comparison in a check region holds."]]],
];
export const glossary = () => openDialog(() => {
  const [q, setQ] = createSignal("");
  return (
    <Dialog title="Glossary" wide>
      <input class="find" placeholder="Search terms" value={q()} onInput={e => setQ(e.currentTarget.value)} />
      <div class="gloss"><For each={GLOSSARY}>{([h, list]) => { const f = () => list.filter(x => (x[0] + x[1]).toLowerCase().includes(q().toLowerCase())); return <Show when={f().length}><h4>{h}</h4><dl><For each={f()}>{([t, d]) => <><dt>{t}</dt><dd>{d}</dd></>}</For></dl></Show>; }}</For></div>
    </Dialog>
  );
});

export const shortcuts = () => openDialog(() => (
  <Dialog title="Keyboard shortcuts">
    <table class="keys"><tbody><For each={[["Ctrl+M", "New math region"], ["Ctrl+T", "New text region"], ["Enter or F2", "Edit the selected region"], ["Esc", "Finish editing"], ["↑ / ↓", "Select the region above or below"],
      ["Alt+↑ / Alt+↓", "Move the region up or down"], ["Delete", "Delete the region"], ["Ctrl+C / Ctrl+V", "Copy and paste regions"], ["Ctrl+Z / Ctrl+Y", "Undo, redo"],
      ["Ctrl+S", "Save"], ["Ctrl+O", "Open"], ["Ctrl+N", "New sheet"], ["Ctrl+P", "Print or save as PDF"], ["Ctrl+K or Alt+Q", "Tell me what you want to do"], ["F1", "This list"]]}>{([k, v]) => <tr><td>{k}</td><td>{v}</td></tr>}</For></tbody></table>
  </Dialog>
));

/** Confirms closing with unsaved changes. */
export const confirmClose = (then: () => void) => openDialog(() => (
  <Dialog title="Save changes?" okText="Save" cancel="Cancel" ok={async () => { const o = await call<Out | null>("save", { asNew: false }); if (o) then(); return !!o; }}
    extra={<button class="btn" onClick={() => { closeDialog(); then(); }}>Don't save</button>}>
    <p>Do you want to save the changes to “{S().name}”?</p>
  </Dialog>
));

/** Every unit (by kind), constant and function; a click types it into the region being edited. */
export const unitsList = (start = "units") => openDialog(() => {
  const [lists] = createResource(() => call<Lists>("lists"));
  const [tab, setTab] = createSignal(start), [q, setQ] = createSignal("");
  const match = (...t: string[]) => t.join(" ").toLowerCase().includes(q().toLowerCase());
  const groups = () => { const m = new Map<string, Lists["units"]>(); for (const u of lists()?.units ?? []) if (match(...u)) m.set(u[2], [...(m.get(u[2]) ?? []), u]); return [...m.entries()]; };
  const ins = (t: string) => { closeDialog(); setTimeout(() => typeText(t)); };
  return (
    <Dialog title="Units, constants and functions" wide>
      <div class="ulhead"><div class="seg"><For each={[["units", "Units"], ["constants", "Constants"], ["functions", "Functions"]]}>{([k, l]) => <button classList={{ on: tab() === k }} onClick={() => setTab(k)}>{l}</button>}</For></div>
        <input class="find" placeholder="Search, such as pressure or psi" value={q()} onInput={e => setQ(e.currentTarget.value)} /></div>
      <p class="muted">Click one to type it into the region you are editing. Units go after a number: <code>120 m^3/h</code>, <code>2.5 kN*m</code>; combine them with * / ^ and brackets.</p>
      <div class="ulist">
        <Show when={tab() === "units"}><For each={groups()}>{([g, us]) => <section><h4>{g}</h4><table><tbody><For each={us}>{u => <tr onClick={() => ins(` ${u[0]}`)}><td class="sym">{u[0]}</td><td>{u[1]}</td><td class="muted">{u[3]}</td></tr>}</For></tbody></table></section>}</For></Show>
        <Show when={tab() === "constants"}><table><tbody><For each={(lists()?.constants ?? []).filter(c => match(...c))}>{c => <tr onClick={() => ins(c[0])}><td class="sym">{c[0]}</td><td>{c[1]}</td></tr>}</For></tbody></table>
          <p class="muted">A name you define on the sheet takes the place of a constant. σ is not a constant (it usually means stress); Stefan–Boltzmann is σ_SB.</p></Show>
        <Show when={tab() === "functions"}><table><tbody><For each={(lists()?.functions ?? []).filter(c => match(...c))}>{c => <tr onClick={() => ins(c[0].replace(/\(.*\)/, "()"))}><td class="sym">{c[0]}</td><td>{c[1]}</td></tr>}</For></tbody></table></Show>
      </div>
    </Dialog>
  );
});
