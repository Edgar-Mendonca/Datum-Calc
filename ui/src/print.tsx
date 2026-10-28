// File › Print: the calc sheet as pages, with the title block on every page, "Page x of y" and the
// references at the end. Regions are measured at the page width, then packed into pages (a region is
// never cut in two). The pages are printed with the WebView's print dialog, where "Save as PDF" makes a PDF.
import { For, Show, JSX, createMemo, createSignal, onMount, onCleanup } from "solid-js";
import { Portal } from "solid-js/web";
import { Region } from "./api";
import { S, out, act } from "./store";
import { Pick } from "./ui";
import { MathView } from "./calc";

/** Paper sizes in millimetres (portrait). */
export const PAPERS: [string, number, number][] = [["A4", 210, 297], ["Letter", 215.9, 279.4], ["A3", 297, 420], ["Legal", 215.9, 355.6]];
export const MM = 96 / 25.4;

/** How a region looks on paper. Shared by the measuring pass, the preview and the print. */
export const PrintRegion = (p: { r: Region }) => (
  <div class={`pr k-${p.r.kind}`} classList={{ [`s-${p.r.style}`]: !!p.r.style }}>
    <Show when={p.r.kind !== "text" && p.r.text.trim()} fallback={<div class="prtext">{p.r.text || " "}</div>}><MathView r={p.r} paper /></Show>
  </div>
);

/** The title block at the top of each printed page. */
const TitleBlock = (p: { page: number; pages: number }) => {
  const t = () => S().title;
  const F = (q: { l: string; v: JSX.Element; w?: number; c?: string }) => <div class={`ptbf ${q.c ?? ""}`} style={{ "grid-column": `span ${q.w ?? 1}` }}><small>{q.l}</small><span>{q.v}</span></div>;
  return (
    <div class="ptb">
      <div class="ptblogo">{t().logo ? <img src={t().logo} alt="" /> : <b>{t().company}</b>}</div>
      <F l="Project" v={t().project} w={3} /><F l="Calc no." v={t().number} c="end" />
      <F l="Title" v={t().title || S().name} w={4} c="end" />
      <F l="Prepared" v={t().by} /><F l="Checked" v={t().checked} /><F l="Date" v={t().date} /><F l="Rev" v={t().rev} c="end" />
      <F l="Client" v={t().client} w={2} c="last" /><F l="Company" v={t().company} c="last" /><F l="Sheet" v={`${p.page} of ${p.pages}`} c="last end" />
    </div>
  );
};

type PageOf = { regions: Region[]; refs: boolean };
const geom = () => { const p = PAPERS.find(x => x[0] === S().page.paper) ?? PAPERS[0]; const m = S().page.margins; return { pw: p[1], ph: p[2], m, W: (p[1] - m[1] - m[3]) * MM, H: (p[2] - m[0] - m[2]) * MM }; };
const TB_H = 124, FOOT_H = 26, GAP = 6;

/** Packs measured regions into pages. References (style "ref") go to the end under their own heading. */
const pack = (heights: Map<number, number>): PageOf[] => {
  const g = geom(), room = g.H - TB_H - FOOT_H - 12;
  const body = S().regions.filter(r => r.style !== "ref"), refs = S().regions.filter(r => r.style === "ref");
  const pages: PageOf[] = [{ regions: [], refs: false }];
  let used = 0;
  const put = (r: Region, refsPage: boolean) => {
    const h = (heights.get(r.id) ?? 20) + GAP;
    let cur = pages[pages.length - 1];
    if (used + h > room && cur.regions.length) { cur = { regions: [], refs: refsPage }; pages.push(cur); used = 0; }
    cur.regions.push(r); used += h;
  };
  body.forEach(r => put(r, false));
  if (refs.length) {
    const cur = pages[pages.length - 1];
    if (used + 40 > room && cur.regions.length) { pages.push({ regions: [], refs: true }); used = 0; } else cur.refs = true;
    used += 34;
    refs.forEach(r => put(r, true));
  }
  return pages;
};

const Page = (p: { pg: PageOf; n: number; pages: number; firstRef: boolean }) => {
  const g = geom();
  return (
    <div class="ppage calcpage" style={{ width: `${g.pw * MM}px`, height: `${g.ph * MM}px` }}>
      <div class="pin" style={{ left: `${g.m[3] * MM}px`, top: `${g.m[0] * MM}px`, width: `${g.W}px`, height: `${g.H}px` }}>
        <TitleBlock page={p.n} pages={p.pages} />
        <div class="pbody">
          <For each={p.pg.regions}>{(r, i) => <>
            <Show when={p.firstRef && r.style === "ref" && (i() === 0 || p.pg.regions[i() - 1].style !== "ref")}><h4 class="prefs">References</h4></Show>
            <PrintRegion r={r} />
          </>}</For>
        </div>
        <div class="pcfoot"><span>{S().title.number || S().name}{S().title.rev ? ` · Rev ${S().title.rev}` : ""}</span><span>Page {p.n} of {p.pages}</span><span>Datum Calc</span></div>
      </div>
    </div>
  );
};

const [printing, setPrinting] = createSignal(false);

export const PrintPage = () => {
  const [heights, setHeights] = createSignal(new Map<number, number>());
  const [zoomP, setZoomP] = createSignal(0.7);
  let meas!: HTMLDivElement;
  const measure = () => { const m = new Map<number, number>(); meas?.querySelectorAll<HTMLElement>("[data-id]").forEach(el => m.set(+el.dataset.id!, el.getBoundingClientRect().height)); setHeights(m); };
  onMount(() => { document.fonts?.ready.then(measure); measure(); });
  const pages = createMemo(() => { out(); return pack(heights()); });
  const firstRef = (k: number) => pages().findIndex(p => p.regions.some(r => r.style === "ref")) === k;
  const printNow = () => {
    const g = geom();
    setPrinting(true);
    const css = document.createElement("style");
    css.textContent = `@page { size: ${g.pw}mm ${g.ph}mm; margin: 0; }`;
    document.head.append(css);
    document.documentElement.classList.add("printing");
    setTimeout(() => { window.print(); document.documentElement.classList.remove("printing"); css.remove(); setPrinting(false); }, 300);
  };
  onCleanup(() => setPrinting(false));
  return (
    <div class="printpage">
      <div class="pset">
        <h1>Print</h1>
        <p class="pwhat">Printing: <b>{S().title.title || S().name}</b><br /><span class="muted">The calc sheet with its title block on every page, page numbers and the references at the end.</span></p>
        <div class="pbtns"><button class="btn pri big" onClick={printNow}><svg class="ic"><use href="#i-print" /></svg> Print / Save as PDF</button></div>
        <p class="muted">In the print window choose your printer, or <b>Save as PDF</b>. {pages().length} {pages().length === 1 ? "page" : "pages"} · {S().page.paper} portrait.</p>
        <h4>Page</h4>
        <label class="pl">Paper</label><Pick value={S().page.paper} set={v => act({ SheetSet: { field: "paper", value: v } }).then(() => requestAnimationFrame(measure))} options={PAPERS.map(p => p[0])} />
        <label class="pl">Margins (mm)</label>
        <div class="prow">{(["Top", "Right", "Bottom", "Left"] as const).map((l, i) => <label>{l}<input type="number" min="0" max="60" value={S().page.margins[i]} onChange={e => {
          const m = [...S().page.margins]; m[i] = Math.min(60, Math.max(0, +e.currentTarget.value)); act({ SheetSet: { field: "margins", value: m } }).then(() => requestAnimationFrame(measure));
        }} /></label>)}</div>
        <p class="muted">The title block is set in Home › Title Block. Regions styled as Reference print at the end under “References”.</p>
        <Show when={!S().regions.length}><p class="warnline">The sheet is empty: only the title block will print.</p></Show>
      </div>
      <div class="pprev">
        <div class="pzoom"><span class="muted">Preview</span><input type="range" min="0.3" max="1.2" step="0.05" value={zoomP()} onInput={e => setZoomP(+e.currentTarget.value)} /><span class="muted">{Math.round(zoomP() * 100)}%</span></div>
        <For each={pages()}>{(pg, k) => <div class="pwrap" style={{ width: `${geom().pw * MM * zoomP()}px`, height: `${geom().ph * MM * zoomP()}px` }}><div style={{ transform: `scale(${zoomP()})`, "transform-origin": "0 0" }}><Page pg={pg} n={k() + 1} pages={pages().length} firstRef={firstRef(k())} /></div></div>}</For>
      </div>
      {/* The measuring pass: every region at the printed width, off screen. */}
      <div class="pmeasure" ref={meas} style={{ width: `${geom().W}px` }}><For each={S().regions}>{r => <div data-id={r.id}><PrintRegion r={r} /></div>}</For></div>
      <Show when={printing()}><Portal mount={document.body}><div id="printroot"><For each={pages()}>{(pg, k) => <Page pg={pg} n={k() + 1} pages={pages().length} firstRef={firstRef(k())} />}</For></div></Portal></Show>
    </div>
  );
};
