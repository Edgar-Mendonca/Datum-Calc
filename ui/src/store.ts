// Window state: the open sheet, the selected region, panels, messages, menus and dialogs.
import { createSignal, batch, JSX } from "solid-js";
import { invoke, Out, Options, Region } from "./api";

export const [out, setOutRaw] = createSignal<Out | null>(null);
export const [tick, setTick] = createSignal(0);
export const setOut = (o: Out | null | undefined) => { if (o) batch(() => { setOutRaw(o); setTick(t => t + 1); }); };
export const S = () => out()!.sheet;

/** While true, folded sections are shown (exports draw the plots in them). */
export const [unfoldAll, setUnfoldAll] = createSignal(false);
const level = (r: Region) => r.kind === "text" ? (r.style === "heading" ? 1 : r.style === "subheading" ? 2 : 0) : 0;
/** The regions on the screen: those under a folded heading are left out until the next heading of its level. */
export const shown = () => {
  const res: Region[] = [];
  let folded = 0;
  for (const r of S().regions) {
    const l = level(r);
    if (!unfoldAll() && folded && (l === 0 || l > folded)) continue;
    folded = l && r.fold ? l : 0;
    res.push(r);
  }
  return res;
};
/** The folded headings a region is under (empty when it is on the screen). */
export const foldedOver = (id: number) => {
  const rs = S().regions, i = rs.findIndex(r => r.id === id), over: Region[] = [];
  let need = 3;
  for (let j = i - 1; j >= 0 && need > 1; j--) { const l = level(rs[j]); if (l && l < need) { if (rs[j].fold) over.push(rs[j]); need = l; } }
  return over;
};
/** A folded heading's section: the heading and the regions under it, as ids. */
export const section = (id: number) => {
  const rs = S().regions, i = rs.findIndex(r => r.id === id), r = rs[i];
  if (i < 0 || !r.fold || !level(r)) return [id];
  let j = i + 1;
  while (j < rs.length && (level(rs[j]) === 0 || level(rs[j]) > level(r))) j++;
  return rs.slice(i, j).map(x => x.id);
};
/** auto, light or dark; auto follows Windows. */
export const [theme, setThemeRaw] = createSignal("auto");
const darkMq = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;
export const applyTheme = () => { document.documentElement.dataset.theme = theme() === "auto" ? (darkMq?.matches ? "dark" : "light") : theme(); setThemeTick(t => t + 1); };
export const [themeTick, setThemeTick] = createSignal(0);
darkMq?.addEventListener("change", () => theme() === "auto" && applyTheme());
export const setTheme = (t: string) => { setThemeRaw(t); applyTheme(); setOpts({ ...opts(), theme: t }); invoke("set_options", { opts: opts() }).catch(() => {}); };
export const [opts, setOpts] = createSignal<Options>({ autosave: 5, beta: false, theme: "auto", openLast: false, units: "SI", digits: 4, paper: "A4", company: "", by: "", logo: "", client: "", checked: "", number: "auto" });

export const [backstage, setBackstage] = createSignal<string | null>("home");
export const [tab, setTab] = createSignal("home");
/** The selected region (its id), and the one being edited. */
export const [selId, setSelId] = createSignal(0);
export const [editId, setEditId] = createSignal(0);
export const [zoom, setZoom] = createSignal(100);
export const [varsPanel, setVarsPanel] = createSignal(true);

// ---------- editing ----------
export const act = async (op: object, quiet = false): Promise<boolean> => {
  try { setOut(await invoke<Out>("apply", { op })); return true; }
  catch (e) { if (!quiet) toast(String(e)); return false; }
};
export const call = async <T = any>(cmd: string, args?: object): Promise<T | undefined> => {
  try { return await invoke<T>(cmd, args); } catch (e) { toast(String(e)); return undefined; }
};
export const loadOut = async (cmd: string, args?: object) => { const o = await call<Out | null>(cmd, args); if (o) { setOut(o); setBackstage(null); setSelId(o.sheet.regions[0]?.id ?? 0); setEditId(0); } return o; };

// ---------- messages, menus, dialogs ----------
export const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
let timer = 0;
export const toast = (text: string, ok = false) => { setMsg({ text, ok }); clearTimeout(timer); timer = window.setTimeout(() => setMsg(null), ok ? 3000 : 7000); };
export type Item = { label: string; run?: () => void; keys?: string; off?: boolean; on?: boolean; sub?: Item[]; icon?: string } | "-";
export const [menu, setMenu] = createSignal<{ x: number; y: number; items: Item[] } | null>(null);
export const openMenu = (e: MouseEvent | { x: number; y: number }, items: Item[]) => {
  if ("preventDefault" in e) e.preventDefault();
  const r = (e as MouseEvent).currentTarget instanceof HTMLElement && (e as MouseEvent).type === "click" ? ((e as MouseEvent).currentTarget as HTMLElement).getBoundingClientRect() : null;
  setMenu({ x: r ? r.left : (e as MouseEvent).clientX ?? (e as any).x, y: r ? r.bottom + 2 : (e as MouseEvent).clientY ?? (e as any).y, items });
};
export const [dialogs, setDialogs] = createSignal<(() => JSX.Element)[]>([]);
export const openDialog = (d: () => JSX.Element) => setDialogs(v => [...v, d]);
export const closeDialog = () => setDialogs(v => v.slice(0, -1));
