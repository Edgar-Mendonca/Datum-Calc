// Calls into the Rust shell and the shapes it returns.
declare global {
  interface Window { __TAURI__?: { core: { invoke: (cmd: string, args?: object) => Promise<any> }; event: { listen: (e: string, f: (x: { payload: any }) => void) => Promise<() => void> } } }
}

export const invoke = <T = any>(cmd: string, args?: object): Promise<T> =>
  window.__TAURI__ ? window.__TAURI__.core.invoke(cmd, args) : Promise.reject("Datum Calc must run inside its desktop window.");
export const listen = (e: string, f: (p: any) => void) => window.__TAURI__?.event.listen(e, x => f(x.payload));

export type Kind = "math" | "text" | "check" | "solve" | "plot" | "table" | "image";
export interface Region { id: number; kind: Kind; text: string; unit: string; style: string; data: any }
export interface TitleBlock { project: string; number: string; title: string; by: string; checked: string; date: string; rev: string; company: string; client: string; logo: string }
export interface Sheet {
  format: number; name: string; title: TitleBlock; page: { paper: string; margins: [number, number, number, number] }; units: "SI" | "US"; digits: number; subst: boolean; polar: boolean;
  regions: Region[]; next_id: number; ui: any;
}
export interface Out { sheet: Sheet; calc: SheetOut; path: string | null; dirty: boolean; undo: boolean; redo: boolean; rev: number }
export interface Options { autosave: number; beta: boolean; theme: string; openLast: boolean; units: string; digits: number; paper: string; company: string; by: string; logo: string; number: string }

export interface CheckOut { pass: boolean; ratio: number | null; lhs: string; rhs: string; op: string }
export interface PlotOut { xs: number[]; xlabel: string; xunit: string; series: { label: string; ys: number[] }[]; yunit: string }
export interface RegionOut {
  id: number; latex: string; result_tex: string; result: string; value: string; unit: string; name: string; input: boolean;
  error: { msg: string; from: number; to: number } | null; warn: string; check: CheckOut | null; plot: PlotOut | null; units: string[]; note: string; subst_tex: string;
  table: TableOut | null; defs: VarOut[];
}
export interface VarOut { name: string; tex: string; value: string; unit: string; id: number }
export interface SheetOut { regions: RegionOut[]; vars: VarOut[]; checks: number; failed: number; errors: number }
export interface Lists { units: [string, string, string, string][]; constants: [string, string][]; functions: [string, string][] }
export interface TableOut { cols: { name: string; tex: string; unit: string; formula: string; cells: string[]; error: string }[]; rows: number }
export interface TableData { cols: { name: string; unit: string; formula: string }[]; rows: string[][] }
export interface PackCard { id: string; name: string; field: string; desc: string; checks: number; regions: number }
