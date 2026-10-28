// What the engine worked out, shown: maths (KaTeX), results with their units, PASS/FAIL badges,
// errors on their line, plots, and the Variables panel. Used on screen and on paper.
import { For, Show, createMemo, onCleanup, onMount } from "solid-js";
import katex from "katex";
import "katex/dist/katex.min.css";
import { Chart, LineController, LineElement, PointElement, LinearScale, Tooltip, Legend } from "chart.js";
import { PlotOut, Region, RegionOut, TableOut } from "./api";
import { out, act, call, openMenu, setSelId, toast } from "./store";
import { Help } from "./help";

Chart.register(LineController, LineElement, PointElement, LinearScale, Tooltip, Legend);

const cache = new Map<string, string>();
/** LaTeX to HTML, cached (the same maths is drawn again on every change). */
export const tex = (s: string) => {
  let h = cache.get(s);
  if (h === undefined) {
    try { h = katex.renderToString(s, { throwOnError: false, output: "html", strict: false }); } catch { h = s; }
    if (cache.size > 4000) cache.clear();
    cache.set(s, h);
  }
  return h;
};
export const Tex = (p: { s: string; class?: string; display?: boolean }) => <span class={`tex ${p.class ?? ""}`} innerHTML={tex(p.display ? `\\displaystyle ${p.s}` : p.s)} />;

/** The engine's output for each region, by id. */
export const outs = createMemo(() => new Map((out()?.calc.regions ?? []).map(r => [r.id, r] as [number, RegionOut])));

const unitMenu = (e: MouseEvent, r: Region, o: RegionOut) => openMenu(e, [
  { label: "Automatic", on: !r.unit, run: () => act({ Set: { id: r.id, field: "unit", value: "" } }) }, "-",
  ...o.units.map(u => ({ label: u, on: r.unit === u, run: () => act({ Set: { id: r.id, field: "unit", value: u } }) })),
]);

/** The text the user typed, with the part in error underlined. */
const Marked = (p: { text: string; from: number; to: number }) => {
  const c = () => [...p.text];
  return <code class="marked">{c().slice(0, p.from).join("")}<mark>{c().slice(p.from, Math.max(p.to, p.from + 1)).join("") || " "}</mark>{c().slice(Math.max(p.to, p.from + 1)).join("")}</code>;
};

export const Badge = (p: { pass: boolean }) => <span class="badge" classList={{ pass: p.pass, fail: !p.pass }}>{p.pass ? "✓ PASS" : "✗ FAIL"}</span>;

/** A math, check, solve or plot region as maths with its result. `paper` drops the screen-only parts. */
export const MathView = (p: { r: Region; paper?: boolean }) => {
  const o = () => outs().get(p.r.id);
  return (
    <Show when={o()} fallback={<div class="rtxt">{p.r.text}</div>}>{o => <>
      <Show when={p.r.text.trim() && p.r.kind === "table"}><div class="tcap">{p.r.text}</div></Show>
      <div class="mline" classList={{ none: p.r.kind === "table" }}>
        <Show when={o().latex}><Tex s={o().latex} display /></Show>
        <Show when={o().result_tex && !o().input && !o().check}><Tex s={o().result_tex.startsWith("\\Rightarrow") ? o().result_tex : `= ${o().result_tex}`} class="res" display /></Show>
        <Show when={!p.paper && o().units.length && !o().check}>
          <button class="unitbtn" title="Show the result in another unit" onMouseDown={e => e.stopPropagation()} onClick={e => { e.stopPropagation(); unitMenu(e, p.r, o()); }}>{p.r.unit ? "unit ▾" : "▾"}</button>
        </Show>
        <Show when={o().note}><span class="note">{o().note}</span></Show>
        <Show when={o().check}>{c => <span class="checkout"><Badge pass={c().pass} /><Show when={c().ratio !== null}><span class="ratio">utilisation <b>{c().ratio!.toFixed(2)}</b></span></Show></span>}</Show>
      </div>
      <Show when={o().check}>{c => <div class="checkvals">{c().lhs} {({ "<=": "≤", ">=": "≥", "!=": "≠", "==": "=" } as Record<string, string>)[c().op] ?? c().op} {c().rhs}</div>}</Show>
      <Show when={o().plot}>{pl => <PlotView p={pl()} id={p.r.id} paper={p.paper} />}</Show>
      <Show when={o().table}>{t => <TableView t={t()} />}</Show>
      <Show when={o().error}>{e => <div class="rerr" role="alert"><span class="ico">⚠</span><span>{e().msg}</span><Show when={!p.paper && e().to > e().from}><Marked text={p.r.text} from={e().from} to={e().to} /></Show></div>}</Show>
      <Show when={o().warn}><div class="rwarn"><span class="ico">⚠</span>{o().warn}</div></Show>
    </>}</Show>
  );
};

const COLORS = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#F0E442", "#000000"];
/** A plot in the family chart style: white plot area, thin axes, Okabe-Ito colours, crosshair tooltip, PNG button. */
export const PlotView = (p: { p: PlotOut; id: number; paper?: boolean }) => {
  let cv!: HTMLCanvasElement;
  let chart: Chart | undefined;
  onMount(() => {
    chart = new Chart(cv, {
      type: "line",
      data: { datasets: p.p.series.map((s, k) => ({ label: s.label, data: p.p.xs.map((x, i) => ({ x, y: Number.isFinite(s.ys[i]) ? s.ys[i] : null })) as any, borderColor: COLORS[k % 8], backgroundColor: COLORS[k % 8], borderWidth: 1.8, pointRadius: 0, spanGaps: false })) },
      options: {
        animation: false, responsive: true, maintainAspectRatio: false, parsing: false, normalized: true,
        interaction: { mode: "index", intersect: false },
        plugins: { legend: { display: p.p.series.length > 1, position: "top", labels: { boxWidth: 14, font: { family: "IBM Plex Sans", size: 11 }, color: "#1B2733" } },
          tooltip: { enabled: !p.paper, callbacks: { title: it => `${p.p.xlabel} = ${(it[0]?.parsed.x ?? 0).toPrecision(4)} ${p.p.xunit}`, label: it => `${it.dataset.label}: ${(it.parsed.y ?? 0).toPrecision(4)} ${p.p.yunit}` } } },
        scales: {
          x: { type: "linear", min: p.p.xs[0], max: p.p.xs[p.p.xs.length - 1], title: { display: true, text: `${p.p.xlabel}${p.p.xunit ? ` (${p.p.xunit})` : ""}`, color: "#1B2733", font: { family: "IBM Plex Sans", size: 11 } }, grid: { color: "#EEF0F2" }, border: { color: "#5B6878" }, ticks: { color: "#3B4859", font: { size: 10 } } },
          y: { title: { display: !!p.p.yunit, text: p.p.yunit, color: "#1B2733", font: { family: "IBM Plex Sans", size: 11 } }, grid: { color: "#EEF0F2" }, border: { color: "#5B6878" }, ticks: { color: "#3B4859", font: { size: 10 } } },
        },
      },
      plugins: [{ id: "cross", afterDraw: c => { const a = (c.tooltip as any)?._active?.[0]; if (!a || p.paper) return; const { ctx, chartArea: { top, bottom } } = c; ctx.save(); ctx.strokeStyle = "#8D99A8"; ctx.setLineDash([3, 3]); ctx.beginPath(); ctx.moveTo(a.element.x, top); ctx.lineTo(a.element.x, bottom); ctx.stroke(); ctx.restore(); } }],
    });
  });
  onCleanup(() => chart?.destroy());
  const png = async () => { const d = cv.toDataURL("image/png"); const path = await call<string | null>("save_file", { name: "plot.png", data: d }); if (path) toast(`Saved ${path}`, true); };
  return (
    <div class="plotbox" classList={{ paper: p.paper }}>
      <Show when={!p.paper}><button class="btn sm pngbtn" onMouseDown={e => e.stopPropagation()} onClick={png}>PNG</button></Show>
      <canvas ref={cv} />
    </div>
  );
};

/** A table region as worked out: headings as maths, units under them, numbers right-aligned. */
export const TableView = (p: { t: TableOut }) => (
  <table class="ctable">
    <thead>
      <tr><For each={p.t.cols}>{c => <th><Show when={c.tex} fallback={<span class="muted">—</span>}><Tex s={c.tex} /></Show></th>}</For></tr>
      <tr class="cu"><For each={p.t.cols}>{c => <th>{c.unit ? `(${c.unit})` : ""}{c.formula ? <span class="fx" title={`= ${c.formula}`}> ƒ</span> : ""}</th>}</For></tr>
    </thead>
    <tbody><For each={Array.from({ length: p.t.rows }, (_, i) => i)}>{i => <tr><For each={p.t.cols}>{c => <td classList={{ calc: !!c.formula }}>{c.cells[i] ?? ""}</td>}</For></tr>}</For></tbody>
  </table>
);

/** A picture region: the picture at its width and the caption under it. */
export const ImageView = (p: { r: Region }) => (
  <figure class="cimg">
    <Show when={p.r.data?.src} fallback={<div class="muted">No picture yet: double-click to choose one, or paste a picture.</div>}>
      <img src={p.r.data.src} alt={p.r.text} style={{ width: `${p.r.data.width ?? 60}%` }} />
    </Show>
    <Show when={p.r.text.trim()}><figcaption>{p.r.text}</figcaption></Show>
  </figure>
);

/** Every name on the sheet with its value and unit; a click selects the line that defines it. */
export const VarsPanel = () => {
  const vars = () => out()?.calc.vars ?? [];
  return (
    <aside class="vars">
      <header>Variables <Help k="vars" /></header>
      <Show when={vars().length} fallback={<p class="muted">Every name the sheet defines appears here with its value and unit.</p>}>
        <table><tbody><For each={vars()}>{v => <tr onClick={() => setSelId(v.id)} title="Go to the line that defines it"><td><Tex s={v.tex} /></td><td class="r">{v.value}</td><td>{v.unit}</td></tr>}</For></tbody></table>
      </Show>
      <Show when={(out()?.calc.checks ?? 0) > 0}>
        <div class="vsum"><b>Checks</b> {out()!.calc.checks - out()!.calc.failed} of {out()!.calc.checks} pass{out()!.calc.failed ? <span class="failn"> · {out()!.calc.failed} fail</span> : ""}</div>
      </Show>
      <Show when={(out()?.calc.errors ?? 0) > 0}><div class="vsum errn">{out()!.calc.errors} line{out()!.calc.errors > 1 ? "s" : ""} with errors</div></Show>
    </aside>
  );
};
