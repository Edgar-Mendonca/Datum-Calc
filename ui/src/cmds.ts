// Commands shared by the ribbon, menus, keys and Tell Me.
import { act, call, loadOut, out, selId, setBackstage, setEditId, setOut, setSelId, setZoom, toast, zoom, setVarsPanel, varsPanel, S } from "./store";
import { Kind, Out } from "./api";
import * as D from "./dialogs";
import { outs } from "./calc";

const GH = "https://github.com/Edgar-Mendonca/Datum-Calc";
/** Where a new region goes: below the selected one, or at the end. */
const below = () => { const i = out()?.sheet.regions.findIndex(r => r.id === selId()) ?? -1; return i >= 0 ? i + 1 : out()?.sheet.regions.length ?? 0; };
const STARTER: Record<Kind, string> = { math: "", text: "", check: "check ", solve: "solve x^2 = 2 for x in 0 .. 10", plot: "plot sin(x), cos(x) for x in 0 .. 2*pi", table: "", image: "" };

/** The textarea being edited, so symbol buttons can type into it. */
let field: HTMLTextAreaElement | null = null;
export const setField = (el: HTMLTextAreaElement | null) => { field = el; };
export const typeText = (t: string, back = 0) => {
  if (!field || !document.body.contains(field)) return toast("Double-click a region to edit it first, then click the symbol.");
  const a = field.selectionStart, b = field.selectionEnd;
  field.setRangeText(t, a, b, "end");
  if (back) field.selectionStart = field.selectionEnd = a + t.length - back;
  field.dispatchEvent(new Event("input", { bubbles: true }));
  field.focus();
};

export const C = {
  insert: async (kind: Kind, text?: string, data: unknown = null) => {
    if (!out()) return;
    const at = below();
    if (kind === "table" && !data) data = { cols: [{ name: "Q", unit: "m^3/h", formula: "" }, { name: "H", unit: "m", formula: "" }], rows: [["0", ""], ["", ""], ["", ""]] };
    if (await act({ Insert: { at, kind, text: text ?? STARTER[kind], data } })) { const r = S().regions[at]; if (r) { setSelId(r.id); if (kind !== "image" || !data) setEditId(r.id); } }
  },
  picture: async (src?: string) => {
    src = src ?? await call<string | null>("pick_image") ?? undefined;
    if (src) C.insert("image", "", { src, width: 60 });
  },
  del: () => { const id = selId(); if (!id) return toast("Select a region first."); const i = S().regions.findIndex(r => r.id === id); act({ Delete: { ids: [id] } }).then(ok => { if (ok) setSelId(S().regions[Math.min(i, S().regions.length - 1)]?.id ?? 0); }); },
  move: (up: boolean) => { const id = selId(); if (id) act({ Move: { ids: [id], up } }); },
  copy: async () => {
    const r = out()?.sheet.regions.find(x => x.id === selId());
    if (!r) return toast("Select a region first.");
    await navigator.clipboard?.writeText(r.text).catch(() => {});
    toast("Copied.", true);
  },
  paste: async (given?: string) => {
    const t = given ?? await navigator.clipboard?.readText().catch(() => "");
    if (!t) return toast("The clipboard has no text.");
    const lines = t.split(/\r?\n/).map(l => l.trim()).filter(Boolean);
    const at = below();
    if (await act({ Batch: lines.map((l, k) => ({ Insert: { at: at + k, kind: /^check\b/.test(l) ? "check" : /[=]/.test(l) ? "math" : "text", text: l } })) })) setSelId(S().regions[at + lines.length - 1]?.id ?? 0);
  },
  undo: async () => setOut(await call<Out>("history", { redo: false })),
  redo: async () => setOut(await call<Out>("history", { redo: true })),
  save: async (asNew = false) => { const o = await call<Out | null>("save", { asNew }); if (o) { setOut(o); toast("Saved.", true); } },
  open: () => loadOut("open", { path: null }),
  newSheet: () => loadOut("new_sheet"),
  pack: (id: string) => loadOut("new_from_pack", { id }),
  /** The selected region's result ("19.61 kW"), to paste into a spreadsheet or a report. */
  copyResult: async () => {
    const o = outs().get(selId());
    const t = o?.check ? o.result : o?.value ? `${o.value}${o.unit ? ` ${o.unit}` : ""}` : "";
    if (!t) return toast("Select a region with a result first.");
    await navigator.clipboard?.writeText(t).catch(() => {});
    toast(`Copied ${t}`, true);
  },
  copyLatex: async () => { const o = outs().get(selId()); if (!o?.latex) return toast("Select a math region first."); await navigator.clipboard?.writeText(o.result_tex && !o.input ? `${o.latex} = ${o.result_tex}` : o.latex).catch(() => {}); toast("Copied as LaTeX.", true); },
  /** Inputs, results and checks as CSV (name, value, unit, kind, description). */
  csv: async () => {
    if (!out()) return;
    const q = (x: string) => /[",\n]/.test(x) ? `"${x.replace(/"/g, '""')}"` : x;
    const rows = [["Name", "Value", "Unit", "Kind", "Description"]];
    for (const r of S().regions) {
      const o = outs().get(r.id);
      if (!o) continue;
      if (o.check) rows.push([r.text.split("#")[0].replace(/^check\s*/, "").trim(), o.check.pass ? "PASS" : "FAIL", o.check.ratio === null ? "" : o.check.ratio.toFixed(3), "check", o.note]);
      else if (o.name && o.value && !o.error) rows.push([o.name, o.value, o.unit, r.style === "input" ? "input" : "result", o.note]);
      for (const d of o.defs ?? []) rows.push([d.name, d.value, d.unit, "table", ""]);
    }
    const path = await call<string | null>("save_file", { name: `${S().name}.csv`, data: rows.map(r => r.map(x => q(x ?? "")).join(",")).join("\r\n") + "\r\n" });
    if (path) toast(`Saved ${path}`, true);
  },
  print: () => { setEditId(0); setBackstage("print"); },
  units: (u: "SI" | "US") => act({ SheetSet: { field: "units", value: u } }),
  zoomIn: () => setZoom(Math.min(200, zoom() + 10)),
  zoomOut: () => setZoom(Math.max(50, zoom() - 10)),
  vars: () => setVarsPanel(!varsPanel()),
  file: (page: string) => setBackstage(page),
  options: () => setBackstage("options"),
  github: (path = "") => call("open_url", { url: GH + path }),
};

/** Every command with a name, for Tell Me and the shortcut list. */
export const ALL_CMDS: [string, () => void, string?][] = [
  ["Insert math region", () => C.insert("math"), "Ctrl+M"], ["Insert text region", () => C.insert("text"), "Ctrl+T"], ["Insert check", () => C.insert("check")], ["Insert table", () => C.insert("table")], ["Insert picture", () => C.picture()], ["Units list", () => D.unitsList()], ["Functions and constants", () => D.unitsList("functions")], ["Insert solve", () => C.insert("solve")], ["Insert plot", () => C.insert("plot")],
  ["Delete region", C.del, "Delete"], ["Move region up", () => C.move(true), "Alt+↑"], ["Move region down", () => C.move(false), "Alt+↓"],
  ["Copy", C.copy, "Ctrl+C"], ["Copy result", C.copyResult], ["Copy as LaTeX", C.copyLatex], ["Export CSV (inputs and results)", C.csv], ["New from pack", () => C.file("new")], ["Paste", () => C.paste(), "Ctrl+V"], ["Undo", C.undo, "Ctrl+Z"], ["Redo", C.redo, "Ctrl+Y"],
  ["Save", () => C.save(), "Ctrl+S"], ["Save as", () => C.save(true)], ["Open", C.open, "Ctrl+O"], ["New sheet", C.newSheet, "Ctrl+N"], ["Print / PDF", C.print, "Ctrl+P"],
  ["Title block", () => D.titleBlock()], ["SI units", () => C.units("SI")], ["US units", () => C.units("US")],
  ["Zoom in", C.zoomIn], ["Zoom out", C.zoomOut], ["Options", C.options], ["Check for updates", () => C.file("about")], ["Roadmap", () => C.file("roadmap")],
  ["Keyboard shortcuts", () => D.shortcuts(), "F1"], ["Getting started", () => D.gettingStarted()], ["Glossary", () => D.glossary()],
  ["GitHub page", () => C.github()], ["Report an issue", () => C.github("/issues/new")],
];
