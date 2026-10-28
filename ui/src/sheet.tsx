// The calc sheet on screen: the page with its title block and regions, top to bottom.
// Click selects a region; double-click or Enter edits it; Esc or a click outside finishes.
import { ErrorBoundary, For, Index, Show, createEffect, createSignal, onMount } from "solid-js";
import { Region, TableData } from "./api";
import { act, call, out, S, selId, setSelId, editId, setEditId, zoom, openMenu } from "./store";
import { C, setField } from "./cmds";
import * as D from "./dialogs";
import { Help } from "./help";
import { PAPERS } from "./print";
import { MathView, ImageView, outs } from "./calc";

const MM = 96 / 25.4;
const PH: Record<string, string> = { math: "Q = 120 m^3/h", check: "check sigma <= sigma_allow", solve: "solve x^2 = 2 for x in 0 .. 10", plot: "plot f(x) for x in 0 .. 10", image: "Caption, such as Free-body diagram of the bracket" };
const KIND: Record<string, string> = { math: "Math", text: "Text", check: "Check", solve: "Solve", plot: "Plot", table: "Table", image: "Picture" };

/** The region editor: a textarea that grows with its text and saves when it loses focus. */
const Editor = (p: { r: Region }) => {
  let el!: HTMLTextAreaElement;
  const fit = () => { el.style.height = "auto"; el.style.height = `${el.scrollHeight}px`; };
  let sent = p.r.text;
  const commit = () => { if (el.value !== sent) { sent = el.value; act({ Set: { id: p.r.id, field: "text", value: el.value } }); } };
  onMount(() => { el.value = p.r.text; fit(); setField(el); el.focus(); el.selectionStart = el.selectionEnd = el.value.length; });
  const key = (e: KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === "Escape") { e.preventDefault(); commit(); setEditId(0); }
    else if (e.key === "Enter" && !["text", "image"].includes(p.r.kind) && !e.shiftKey) {
      // Enter ends an equation and starts the next one below it.
      e.preventDefault(); commit(); setEditId(0); C.insert(p.r.kind === "check" ? "check" : "math");
    } else if (e.key === "Enter" && e.ctrlKey) { e.preventDefault(); commit(); setEditId(0); }
  };
  return <textarea ref={el} class="redit" spellcheck={p.r.kind === "text"} rows="1" onInput={fit} onKeyDown={key}
    onBlur={e => { commit(); if (editId() === p.r.id && !(e.relatedTarget as HTMLElement | null)?.closest(".top, .ribbon")) setEditId(0); }} placeholder={PH[p.r.kind] ?? "Text, a heading or a reference"} />;
};

/** The table editor: names, units and formulas over the cells. Edits stay local until it closes, so typing
 *  from cell to cell is not interrupted; Esc, Done or a click outside saves them as one change. */
const TableEditor = (p: { r: Region }) => {
  const start = (): TableData => {
    const d = p.r.data ?? {};
    const cols = Array.isArray(d.cols) && d.cols.length ? d.cols.map((c: any) => ({ name: String(c?.name ?? ""), unit: String(c?.unit ?? ""), formula: String(c?.formula ?? "") })) : [{ name: "x", unit: "", formula: "" }, { name: "y", unit: "", formula: "" }];
    const rows = Array.isArray(d.rows) && d.rows.length ? d.rows.map((r: any) => cols.map((_: unknown, j: number) => String(Array.isArray(r) ? r[j] ?? "" : ""))) : [["", ""], ["", ""], ["", ""]].map(r => cols.map((_: unknown, j: number) => r[j] ?? ""));
    return { cols, rows };
  };
  const [d, setD] = createSignal<TableData>(start());
  const [cap, setCap] = createSignal(p.r.text);
  let el!: HTMLDivElement, done = false;
  const commit = () => {
    if (done) return; done = true;
    const ops: object[] = [];
    if (JSON.stringify(d()) !== JSON.stringify(p.r.data)) ops.push({ Set: { id: p.r.id, field: "data", value: d() } });
    if (cap() !== p.r.text) ops.push({ Set: { id: p.r.id, field: "text", value: cap() } });
    if (ops.length) act({ Batch: ops });
    setEditId(0);
  };
  const col = (j: number, k: "name" | "unit" | "formula", v: string) => setD({ ...d(), cols: d().cols.map((c, i) => i === j ? { ...c, [k]: v } : c) });
  const cell = (i: number, j: number, v: string) => setD({ ...d(), rows: d().rows.map((r, a) => a === i ? r.map((x, b) => b === j ? v : x) : r) });
  const addRow = () => setD({ ...d(), rows: [...d().rows, d().cols.map(() => "")] });
  const addCol = () => setD({ cols: [...d().cols, { name: `c${d().cols.length + 1}`, unit: "", formula: "" }], rows: d().rows.map(r => [...r, ""]) });
  const delRow = () => d().rows.length > 1 && setD({ ...d(), rows: d().rows.slice(0, -1) });
  const delCol = () => d().cols.length > 1 && setD({ cols: d().cols.slice(0, -1), rows: d().rows.map(r => r.slice(0, -1)) });
  onMount(() => (el.querySelector("input") as HTMLInputElement | null)?.focus());
  const key = (e: KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === "Escape") { e.preventDefault(); commit(); }
    else if (e.key === "Enter") {
      // Enter moves down a row (adding one at the bottom), as in a spreadsheet.
      e.preventDefault();
      const t = e.target as HTMLInputElement, i = +(t.dataset.r ?? -1), j = +(t.dataset.c ?? -1);
      if (i < 0) return;
      if (i + 1 >= d().rows.length) addRow();
      requestAnimationFrame(() => (el.querySelector(`input[data-r="${i + 1}"][data-c="${j}"]`) as HTMLInputElement | null)?.focus());
    }
  };
  return (
    <div class="teditor" ref={el} onKeyDown={key} onFocusOut={e => { const t = e.relatedTarget as HTMLElement | null; if (!t || (!el.contains(t) && !t.closest(".top, .ribbon"))) commit(); }}>
      <input class="tcapin" placeholder="Caption (optional), such as Pump curve from the data sheet" value={cap()} onInput={e => setCap(e.currentTarget.value)} />
      <table class="ctable edit">
        <thead>
          <tr><th class="rl">Name</th><Index each={d().cols}>{(c, j) => <th><input value={c().name} placeholder="name" onInput={e => col(j, "name", e.currentTarget.value)} /></th>}</Index></tr>
          <tr><th class="rl">Unit</th><Index each={d().cols}>{(c, j) => <th><input value={c().unit} placeholder="m^3/h" onInput={e => col(j, "unit", e.currentTarget.value)} /></th>}</Index></tr>
          <tr><th class="rl">Formula</th><Index each={d().cols}>{(c, j) => <th><input value={c().formula} placeholder="typed values" title="A formula fills the column from the columns to its left, such as rho*g*Q*H" onInput={e => col(j, "formula", e.currentTarget.value)} /></th>}</Index></tr>
        </thead>
        <tbody><Index each={d().rows}>{(r, i) => <tr><th class="rl">{i + 1}</th><Index each={r()}>{(x, j) => <td><input data-r={i} data-c={j} value={x()} disabled={!!d().cols[j]?.formula.trim()} placeholder={d().cols[j]?.formula.trim() ? "ƒ" : ""} onInput={e => cell(i, j, e.currentTarget.value)} /></td>}</Index></tr>}</Index></tbody>
      </table>
      <div class="tbtns"><button class="btn sm" onClick={addRow}>+ Row</button><button class="btn sm" onClick={delRow}>− Row</button><button class="btn sm" onClick={addCol}>+ Column</button><button class="btn sm" onClick={delCol}>− Column</button>
        <span class="muted">Each column becomes a vector named by its heading. A cell without a unit takes the column's unit.</span><span class="grow" /><button class="btn sm pri" onClick={commit}>Done</button></div>
    </div>
  );
};

/** Picture size choices for the Format tab. */
export const pickPicture = async (id: number) => { const src = await call<string | null>("pick_image"); if (src) act({ Set: { id, field: "data", value: { ...(S().regions.find(r => r.id === id)?.data ?? {}), src } } }); };

const RegionView = (p: { r: Region }) => {
  const editing = () => editId() === p.r.id;
  const empty = () => <div class="rtxt"><span class="muted">Empty {KIND[p.r.kind].toLowerCase()} region: double-click to type.</span></div>;
  return (
    <div class={`region k-${p.r.kind}`} classList={{ sel: selId() === p.r.id, editing: editing(), [`s-${p.r.style}`]: !!p.r.style }} data-id={p.r.id}
      onMouseDown={e => { if (!editing()) { e.preventDefault(); setSelId(p.r.id); (document.querySelector(".cpage") as HTMLElement | null)?.focus(); } }}
      onDblClick={() => { setSelId(p.r.id); setEditId(p.r.id); }}
      onContextMenu={e => { if (editing()) return; setSelId(p.r.id); openMenu(e, [
        { label: "Edit", keys: "Enter", run: () => setEditId(p.r.id) }, "-",
        { label: "Copy Result", run: C.copyResult, off: !outs().get(p.r.id)?.value && !outs().get(p.r.id)?.check }, { label: "Copy as LaTeX", run: C.copyLatex, off: !outs().get(p.r.id)?.latex }, { label: "Copy Text", keys: "Ctrl+C", run: C.copy }, "-",
        { label: "Insert Math Below", run: () => C.insert("math") }, { label: "Insert Text Below", run: () => C.insert("text") }, "-",
        { label: "Mark as Input", on: p.r.style === "input", run: () => act({ Set: { id: p.r.id, field: "style", value: p.r.style === "input" ? "" : "input" } }) },
        { label: "Move Up", keys: "Alt+↑", run: () => C.move(true) }, { label: "Move Down", keys: "Alt+↓", run: () => C.move(false) }, { label: "Delete", keys: "Delete", icon: "del", run: C.del },
      ]); }}>
      <span class="rtag">{KIND[p.r.kind]}</span>
      <Show when={editing()} fallback={
        p.r.kind === "image" ? <ImageView r={p.r} />
          : p.r.kind === "table" ? <MathView r={p.r} />
          : !p.r.text.trim() ? empty()
          : p.r.kind === "text" ? <div class="rtxt">{p.r.text}</div>
          : <MathView r={p.r} />}>
        <Show when={p.r.kind === "table"} fallback={<>
          <Show when={p.r.kind === "image"}><ImageView r={p.r} /><div class="imgbtns"><button class="btn sm" onMouseDown={e => e.preventDefault()} onClick={() => pickPicture(p.r.id)}>Choose picture…</button><span class="muted">Type the caption below.</span></div></Show>
          <Editor r={p.r} />
          <Show when={outs().get(p.r.id)?.error}>{e => <div class="rerr live"><span class="ico">⚠</span>{e().msg}</div>}</Show>
        </>}><TableEditor r={p.r} /></Show>
      </Show>
    </div>
  );
};

const TitleBlockView = () => {
  const t = () => S().title;
  const F = (q: { l: string; v: string; w?: number; c?: string }) => <div class={`tbf ${q.c ?? ""}`} style={{ "grid-column": `span ${q.w ?? 1}` }}><small>{q.l}</small><span>{q.v || "—"}</span></div>;
  return (
    <div class="tblock" title="Click to edit the title block" onClick={() => D.titleBlock()}>
      <div class="tblogo">{t().logo ? <img src={t().logo} alt="" /> : <b>{t().company || "Company"}</b>}</div>
      <F l="Project" v={t().project} w={3} /><F l="Calc no." v={t().number} c="end" />
      <F l="Title" v={t().title || S().name} w={4} c="end" />
      <F l="By" v={t().by} c="last" /><F l="Checked" v={t().checked} c="last" /><F l="Date" v={t().date} c="last" /><F l="Rev" v={t().rev} c="last end" />
    </div>
  );
};

export const SheetView = () => {
  let page!: HTMLDivElement;
  const width = () => (PAPERS.find(p => p[0] === S().page.paper) ?? PAPERS[0])[1] * MM;
  const idx = () => S().regions.findIndex(r => r.id === selId());
  const key = (e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey) return;
    const rs = S().regions, i = idx();
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (e.altKey) return C.move(e.key === "ArrowUp");
      const j = Math.max(0, Math.min(rs.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)));
      if (rs[j]) setSelId(rs[j].id);
    } else if ((e.key === "Enter" || e.key === "F2") && i >= 0) { e.preventDefault(); setEditId(rs[i].id); }
    else if (e.key === "Delete" && i >= 0) { e.preventDefault(); C.del(); }
  };
  // Keep the selected region in view.
  createEffect(() => { const id = selId(); out(); queueMicrotask(() => page?.querySelector(`[data-id="${id}"]`)?.scrollIntoView({ block: "nearest" })); });
  return (
    <div class="canvas" onMouseDown={e => { if (e.target === e.currentTarget) setEditId(0); }}>
      <div class="pagewrap" style={{ zoom: zoom() / 100 }}>
        <div class="cpage" ref={page} tabindex="0" onKeyDown={key} style={{ width: `${width()}px`, padding: `${S().page.margins[0] * MM}px ${S().page.margins[1] * MM}px ${S().page.margins[2] * MM}px ${S().page.margins[3] * MM}px` }}>
          <TitleBlockView />
          <Show when={S().regions.length} fallback={
            <div class="cempty">
              <h3>An empty calc sheet <Help k="page" /></h3>
              <p>Write the calculation from top to bottom: an equation per line with its units, text for headings and references, and checks against your limits.</p>
              <p><button class="btn pri" onClick={() => C.insert("math")}>Math region</button> <button class="btn" onClick={() => C.insert("text")}>Text region</button> <button class="btn" onClick={() => C.insert("check")}>Check</button></p>
            </div>}>
            <For each={S().regions}>{r => <ErrorBoundary fallback={e => <div class="region rerr" data-id={r.id} onClick={() => setSelId(r.id)}><span class="ico">⚠</span>This region could not be shown ({String(e).slice(0, 120)}). Its text is kept: select it and press Enter to edit it.</div>}><RegionView r={r} /></ErrorBoundary>}</For>
          </Show>
        </div>
      </div>
    </div>
  );
};
