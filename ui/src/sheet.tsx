// The calc sheet on screen: the page with its title block and regions, top to bottom.
// Click selects a region; double-click or Enter edits it; Esc or a click outside finishes.
import { For, Show, createEffect, onMount } from "solid-js";
import { Region } from "./api";
import { act, out, S, selId, setSelId, editId, setEditId, zoom } from "./store";
import { C, setField } from "./cmds";
import * as D from "./dialogs";
import { Help } from "./help";
import { PAPERS } from "./print";
import { MathView, outs } from "./calc";

const MM = 96 / 25.4;
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
    else if (e.key === "Enter" && p.r.kind !== "text" && !e.shiftKey) {
      // Enter ends an equation and starts the next one below it.
      e.preventDefault(); commit(); setEditId(0); C.insert(p.r.kind === "check" ? "check" : "math");
    } else if (e.key === "Enter" && e.ctrlKey) { e.preventDefault(); commit(); setEditId(0); }
  };
  return <textarea ref={el} class="redit" spellcheck={p.r.kind === "text"} rows="1" onInput={fit} onKeyDown={key}
    onBlur={e => { commit(); if (editId() === p.r.id && !(e.relatedTarget as HTMLElement | null)?.closest(".top, .ribbon")) setEditId(0); }} placeholder={p.r.kind === "math" ? "Q = 120 m^3/h" : p.r.kind === "check" ? "check sigma <= sigma_allow" : "Text, a heading or a reference"} />;
};

const RegionView = (p: { r: Region }) => (
  <div class={`region k-${p.r.kind}`} classList={{ sel: selId() === p.r.id, editing: editId() === p.r.id, [`s-${p.r.style}`]: !!p.r.style }} data-id={p.r.id}
    onMouseDown={e => { if (editId() !== p.r.id) { e.preventDefault(); setSelId(p.r.id); (document.querySelector(".cpage") as HTMLElement | null)?.focus(); } }}
    onDblClick={() => { setSelId(p.r.id); setEditId(p.r.id); }}>
    <span class="rtag">{KIND[p.r.kind]}</span>
    <Show when={editId() === p.r.id} fallback={
      <Show when={p.r.text.trim()} fallback={<div class="rtxt"><span class="muted">Empty {KIND[p.r.kind].toLowerCase()} region: double-click to type.</span></div>}>
        <Show when={p.r.kind !== "text"} fallback={<div class="rtxt">{p.r.text}</div>}><MathView r={p.r} /></Show>
      </Show>}>
      <Editor r={p.r} />
      <Show when={outs().get(p.r.id)?.error}>{e => <div class="rerr live"><span class="ico">⚠</span>{e().msg}</div>}</Show>
    </Show>
  </div>
);

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
            <For each={S().regions}>{r => <RegionView r={r} />}</For>
          </Show>
        </div>
      </div>
    </div>
  );
};
