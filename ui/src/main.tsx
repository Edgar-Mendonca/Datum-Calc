// Datum Calc window: layout, sidebar, status bar, keyboard shortcuts, startup and recovery.
import { render } from "solid-js/web";
import { For, Show, onMount } from "solid-js";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "./styles.css";
import { invoke, listen, Out, Options } from "./api";
import { out, setOut, backstage, setBackstage, setOpts, opts, dialogs, call, zoom, setZoom, openDialog, toast, setThemeRaw, applyTheme, editId, varsPanel } from "./store";
import { C } from "./cmds";
import { TitleBar, Ribbon, palette } from "./ribbon";
import { Backstage, update, checkUpdate } from "./backstage";
import { SheetView } from "./sheet";
import { VarsPanel } from "./calc";
import { Menu, Toast, Icon, Dialog } from "./ui";
import * as D from "./dialogs";

const SIDE: [string, string, string, () => void][] = [
  ["setup", "setup", "Setup", () => D.titleBlock()], ["sheet", "math", "Sheet", () => setBackstage(null)], ["print", "print", "Print", () => C.print()],
];

const Status = () => (
  <footer class="status">
    <span>Ready</span>
    <span class="muted">{out() ? `${out()!.sheet.regions.length} regions · ${out()!.sheet.units} units · ${out()!.sheet.page.paper}` : ""}</span>
    <Show when={out()?.calc.checks}><span classList={{ late: !!out()?.calc.failed }}>{out()!.calc.checks - out()!.calc.failed}/{out()!.calc.checks} checks pass</span></Show>
    <Show when={out()?.calc.errors}><span class="late">{out()!.calc.errors} with errors</span></Show>
    <span class="grow" />
    <button onClick={C.zoomOut}>−</button><input class="zoom" type="range" min="50" max="200" step="10" value={zoom()} onInput={e => setZoom(+e.currentTarget.value)} /><button onClick={C.zoomIn}>+</button><span class="muted">{zoom()} %</span>
  </footer>
);

const App = () => {
  onMount(async () => {
    try {
      const o = await invoke<Options>("options"); setOpts(o); setThemeRaw(o.theme || "auto"); applyTheme();
      setOut(await invoke<Out>("state"));
      const start = await invoke<string | null>("startup_file");
      if (start) await call("open", { path: start }).then(r => { if (r) { setOut(r as Out); setBackstage(null); } });
      const rec = await invoke<{ name: string; saved: string } | null>("recovery");
      if (rec) openDialog(() => (
        <Dialog title="Recover unsaved work?" okText="Recover" cancel="Discard" ok={async () => { const r = await call<Out>("recover", { keep: true }); if (r) { setOut(r); setBackstage(null); } }}>
          <p>Datum Calc closed before “{rec.name}” was saved. A copy from {rec.saved} was kept.</p>
        </Dialog>
      ));
      checkUpdate(true);
      setInterval(() => { if (opts().autosave > 0) invoke("autosave").catch(() => {}); }, Math.max(1, opts().autosave) * 60000);
      listen("close-requested", () => D.confirmClose(() => invoke("quit")));
    } catch (e) { toast(String(e)); }
  });
  const keys = (e: KeyboardEvent) => {
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase();
    if (dialogs().length) return;
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
    if ((ctrl && k === "k") || (e.altKey && k === "q")) { e.preventDefault(); palette(); }
    else if (ctrl && k === "s") { e.preventDefault(); C.save(e.shiftKey); }
    else if (ctrl && k === "o") { e.preventDefault(); C.open(); }
    else if (ctrl && k === "n") { e.preventDefault(); setBackstage("home"); }
    else if (ctrl && k === "p") { e.preventDefault(); C.print(); }
    else if (ctrl && k === "m" && out()) { e.preventDefault(); C.insert("math"); }
    else if (ctrl && k === "t" && out()) { e.preventDefault(); C.insert("text"); }
    else if (ctrl && k === "z" && !typing) { e.preventDefault(); C.undo(); }
    else if (ctrl && k === "y" && !typing) { e.preventDefault(); C.redo(); }
    else if (ctrl && k === "c" && !typing && !backstage() && !editId()) { e.preventDefault(); C.copy(); }
    else if (ctrl && k === "v" && !typing && !backstage() && !editId()) { e.preventDefault(); C.paste(); }
    else if (e.key === "F1") { e.preventDefault(); D.shortcuts(); }
    else if (e.key === "Escape" && backstage() && out()) setBackstage(null);
  };
  return (
    <div class="app" ref={() => document.addEventListener("keydown", keys)}>
      <TitleBar update={update()?.available ? update()!.latest : null} />
      <Ribbon />
      <main class="work">
        <nav class="side"><For each={SIDE}>{x => <button classList={{ on: x[0] === "sheet" }} onClick={x[3]} title={x[2]}><Icon n={x[1]} /><span>{x[2]}</span></button>}</For></nav>
        <div class="content"><Show when={out()}><SheetView /><Show when={varsPanel()}><VarsPanel /></Show></Show></div>
      </main>
      <Status />
      <Show when={backstage()}><Backstage /></Show>
      <For each={dialogs()}>{d => d()}</For>
      <Menu /><Toast />
    </div>
  );
};

render(() => <App />, document.getElementById("app")!);
