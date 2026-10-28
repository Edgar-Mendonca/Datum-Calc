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
  format: number; name: string; title: TitleBlock; page: { paper: string; margins: [number, number, number, number] }; units: "SI" | "US"; digits: number;
  regions: Region[]; next_id: number; ui: any;
}
export interface Out { sheet: Sheet; path: string | null; dirty: boolean; undo: boolean; redo: boolean; rev: number }
export interface Options { autosave: number; beta: boolean; theme: string; openLast: boolean; units: string; digits: number; paper: string; company: string; by: string; logo: string; number: string }
