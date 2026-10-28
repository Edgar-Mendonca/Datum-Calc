// File (backstage): home with new sheets and recent files, export, print, options, updates, roadmap.
import { For, Show, createSignal, createResource, onMount, JSX } from "solid-js";
import { PrintPage, PAPERS } from "./print";
import { RoadmapPage } from "./roadmap";
import { Options, Out } from "./api";
import { backstage, setBackstage, call, loadOut, out, setOut, opts, setOpts, toast, setThemeRaw, applyTheme } from "./store";
import { C } from "./cmds";
import * as D from "./dialogs";
import { Icon, Text, Pick, Check } from "./ui";
import { listen, invoke } from "./api";

type Info = { current: string; latest: string | null; beta: boolean; available: boolean; notes: string; published: string; url: string | null; sums: string | null; size: number; page: string };
export const [update, setUpdate] = createSignal<Info | null>(null);
export const [checking, setChecking] = createSignal(false), [checkedAt, setCheckedAt] = createSignal("");
export const checkUpdate = async (quiet: boolean) => {
  setChecking(true);
  try { setUpdate(await invoke<Info>("check_update")); setCheckedAt(new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })); if (!quiet) toast(update()?.available ? `Version ${update()!.latest} is available.` : "You have the latest version.", true); }
  catch (e) { if (!quiet) toast(`Could not check for updates: ${e}`); }
  finally { setChecking(false); }
};
/** Links to the project on GitHub: code, releases (change log) and issues (report a problem). */
export const GitHubLinks = () => (
  <div class="ghlinks">
    <For each={[["", "GitHub", "The project and its code"], ["/releases", "Releases", "Every version and its change log"], ["/issues/new", "Report an issue", "Tell us about a bug or an idea"]]}>{([path, l, t]) =>
      <button class="btn" title={t} onClick={() => C.github(path)}><svg class="ic"><use href={path === "/issues/new" ? "#i-bug" : path ? "#i-tag" : "#i-github"} /></svg>{l}</button>}</For>
  </div>
);

/** Release notes: headings, bullets and bold, enough for the changelog. */
const md = (s: string) => s.split("\n").map(l => {
  const e = l.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>").replace(/`(.+?)`/g, "<code>$1</code>");
  return l.startsWith("## ") ? `<h3>${e.slice(3)}</h3>` : l.startsWith("- ") ? `<li>${e.slice(2)}</li>` : e.trim() ? `<p>${e}</p>` : "";
}).join("");

const Home = () => {
  const [recent] = createResource(() => call<{ path: string; name: string; modified: string }[]>("recent"));
  return (
    <div class="home">
      <div>
        <h1>Good {new Date().getHours() < 12 ? "morning" : new Date().getHours() < 18 ? "afternoon" : "evening"}</h1>
        <div class="tiles">
          <button class="tile" onClick={C.newSheet}><div class="pic"><svg class="ic"><use href="#i-new" /></svg></div><b>Blank Sheet</b><span>An empty calc sheet with your title block.</span></button>
          <button class="tile" onClick={C.open}><div class="pic"><svg class="ic"><use href="#i-open" /></svg></div><b>Open</b><span>A Datum Calc sheet (.dcalc) on this computer.</span></button>
          <button class="tile" onClick={() => D.gettingStarted()}><div class="pic"><svg class="ic"><use href="#i-report" /></svg></div><b>Getting Started</b><span>Write the equations, check the results, print the calc sheet.</span></button>
          <button class="tile" onClick={() => setBackstage("roadmap")}><div class="pic"><svg class="ic"><use href="#i-list" /></svg></div><b>Roadmap</b><span>Everything Datum Calc will do, and when.</span></button>
        </div>
        <h2>Recent</h2>
        <table class="recent"><tbody>
          <For each={recent() ?? []} fallback={<tr><td class="muted">Sheets you open or save appear here.</td></tr>}>{r => <tr class="f" onClick={() => loadOut("open", { path: r.path })}><td><b>{r.name}</b><div class="p">{r.path}</div></td><td class="p">{r.modified}</td></tr>}</For>
        </tbody></table>
      </div>
      <UpdateCard />
    </div>
  );
};

const UpdateCard = () => {
  const [pct, setPct] = createSignal<number | null>(null);
  const [ver] = createResource(() => call<string>("version"));
  onMount(() => { listen("update-progress", (p: number) => setPct(p)); if (!update()) checkUpdate(true); });
  const install = async () => { const u = update(); if (!u?.url) return; setPct(0); await call("install_update", { url: u.url, sums: u.sums }); setPct(null); };
  return (
    <div class="card">
      <div class="brand"><img src="logo.svg" alt="" /><div><b>Datum Calc</b><span class="muted">Version {ver()}{opts().beta ? " · beta channel" : ""}</span></div></div>
      <Show when={update()} fallback={<><span class="pill">{checking() ? "Checking for updates…" : "Not checked yet"}</span><button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check for updates"}</button><GitHubLinks /></>}>{u => <>
        <Show when={u().available} fallback={<span class="pill ok">You have the latest version</span>}><span class="pill warn">Version {u().latest}{u().beta ? " (beta)" : ""} is ready</span></Show>
        <Show when={u().available && u().url}><button class="btn pri wide" disabled={pct() !== null} onClick={install}>{pct() === null ? `Update now (${Math.max(1, Math.round(u().size / 1048576))} MB)` : `Downloading… ${pct()}%`}</button></Show>
        <Show when={pct() !== null}><div class="prog"><i style={{ width: `${pct()}%` }} /></div></Show>
        <button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check again"}</button>
        <Show when={checkedAt() && !checking()}><span class="muted small">Checked at {checkedAt()}</span></Show>
        <GitHubLinks />
        <div class="notes"><h4>What's new{u().latest ? ` in ${u().latest}` : ""}</h4><div innerHTML={md(u().notes || "Release notes will appear here.")} /></div>
      </>}</Show>
    </div>
  );
};

/** The sheet as plain text, one region per line (for pasting into a report or an e-mail). */
const asText = () => out()!.sheet.regions.map(r => r.text).join("\n");
const Export = () => (
  <div><h1>Export</h1>
    <div class="tiles">
      <button class="tile" onClick={() => setBackstage("print")}><div class="pic"><svg class="ic"><use href="#i-print" /></svg></div><b>PDF</b><span>Print the calc sheet and choose “Save as PDF” as the printer.</span></button>
      <button class="tile" onClick={async () => { await navigator.clipboard?.writeText(asText()).catch(() => {}); toast("The sheet was copied as text.", true); }}><div class="pic"><svg class="ic"><use href="#i-copy" /></svg></div><b>Copy as text</b><span>Every region as a line of text, to paste anywhere.</span></button>
      <button class="tile" onClick={async () => { const p = await call<string | null>("save_file", { name: `${out()!.sheet.name}.txt`, data: asText() }); if (p) toast(`Saved ${p}`, true); }}><div class="pic"><svg class="ic"><use href="#i-export" /></svg><small>.txt</small></div><b>Text file</b><span>The same text, saved as a file.</span></button>
    </div>
  </div>
);

/** One option: name and a one-line explanation on the left, the control on the right. */
const Opt = (p: { l: string; hint?: string; children: JSX.Element }) => <div class="opt"><div><b>{p.l}</b>{p.hint && <small>{p.hint}</small>}</div><div class="ctl">{p.children}</div></div>;
const Sec = (p: { t: string; about: string; children: JSX.Element }) => <section class="osec"><h3>{p.t}</h3><p class="muted">{p.about}</p>{p.children}</section>;

const OptionsPage = () => {
  const [o, setO] = createSignal<Options>({ ...opts() });
  const save = async () => { const r = await call<Out>("set_options", { opts: o() }); if (r) { setOpts(o()); setOut(r); toast("Options saved.", true); } };
  const put = <K extends keyof Options>(k: K, v: Options[K]) => setO({ ...o(), [k]: v });
  return (
    <div class="options"><h1>Options</h1>
      <p class="muted">Settings for Datum Calc on this computer. Defaults for new sheets apply when a sheet is created; change an open sheet from Home › Sheet.</p>
      <Sec t="General" about="How the window and numbers look.">
        <Opt l="Theme" hint="Auto follows the Windows light or dark setting. Printed sheets are always black on white."><Pick value={o().theme || "auto"} set={v => { put("theme", v); setThemeRaw(v); applyTheme(); }} options={[["auto", "Auto (follow Windows)"], ["light", "Light"], ["dark", "Dark"]]} /></Opt>
        <Opt l="Number format" hint="How results are written. Engineering notation uses powers of ten in steps of three (12.5×10³)."><Pick value={o().number || "auto"} set={v => put("number", v)} options={[["auto", "Plain, engineering for very large or small numbers"], ["eng", "Always engineering notation"]]} /></Opt>
      </Sec>
      <Sec t="Start-up and saving" about="What happens when Datum Calc starts, and how your work is protected.">
        <Opt l="When Datum Calc starts" hint="Open the File page, or reopen the sheet you used last."><Pick value={o().openLast ? "last" : "home"} set={v => put("openLast", v === "last")} options={[["home", "Show the File page"], ["last", "Reopen the last sheet"]]} /></Opt>
        <Opt l="Recovery copy every" hint="A copy is kept while you work, so a crash or power cut loses little. 0 turns it off."><span class="inline"><input type="number" min="0" style={{ width: "70px" }} value={o().autosave} onChange={e => put("autosave", +e.currentTarget.value)} /> minutes</span></Opt>
      </Sec>
      <Sec t="New sheets" about="Defaults given to every new sheet. Each sheet keeps its own settings afterwards.">
        <Opt l="Unit system" hint="The units results are shown in when you have not chosen one."><Pick value={o().units} set={v => put("units", v)} options={[["SI", "SI (m, kg, s, N, Pa, W)"], ["US", "US customary (ft, lb, psi, hp, °F)"]]} /></Opt>
        <Opt l="Significant figures" hint="How many figures results show (2 to 10)."><input type="number" min="2" max="10" style={{ width: "70px" }} value={o().digits} onChange={e => put("digits", Math.min(10, Math.max(2, +e.currentTarget.value)))} /></Opt>
        <Opt l="Paper" hint="Paper for printing new sheets."><Pick value={o().paper || "A4"} set={v => put("paper", v)} options={PAPERS.map(p => p[0])} /></Opt>
      </Sec>
      <Sec t="Title block" about="Filled in on every new sheet. Change a sheet's own title block from Home › Title Block.">
        <Opt l="Company" hint="Printed in the title block."><Text value={o().company} set={v => put("company", v)} /></Opt>
        <Opt l="Prepared by" hint="Your name or initials."><Text value={o().by} set={v => put("by", v)} /></Opt>
        <Opt l="Company logo" hint="PNG or JPG, printed at the top left of each page."><span class="inline">{o().logo && <img src={o().logo} class="logoprev" />}<button class="btn sm" onClick={async () => { const l = await call<string | null>("pick_image"); if (l) put("logo", l); }}>Choose picture…</button>{o().logo && <button class="btn sm" onClick={() => put("logo", "")}>Remove</button>}</span></Opt>
      </Sec>
      <Sec t="Updates" about="Datum Calc checks GitHub for new versions when it starts. Updates install from File › Updates.">
        <Opt l="Beta versions" hint="Get new features early. They may have bugs."><Check label="Get beta versions" value={o().beta} set={v => put("beta", v)} /></Opt>
      </Sec>
      <p class="osave"><button class="btn pri" onClick={save}>Save options</button><span class="muted">Datum Calc keeps everything on this computer. Its only connection is the update check to GitHub.</span></p>
    </div>
  );
};

export const Backstage = () => (
  <div class="bs">
    <aside>
      <button class="back" onClick={() => out() && setBackstage(null)} title="Back">←</button>
      <For each={[["home", "home", "Home"], ["new", "new", "New"], ["open", "open", "Open"], ["save", "save", "Save"], ["saveas", "save", "Save As"], ["export", "export", "Export"], ["print", "print", "Print"], ["roadmap", "list", "Roadmap"]]}>{([k, i, l]) =>
        <button classList={{ on: backstage() === k }} onClick={() => {
          if (k === "open") C.open(); else if (k === "save") C.save(); else if (k === "saveas") C.save(true); else if (k === "new") C.newSheet();
          else setBackstage(k);
        }}><Icon n={i} size={22} />{l}</button>}</For>
      <span class="grow" />
      <button classList={{ on: backstage() === "options" }} onClick={() => setBackstage("options")}><Icon n="opts" size={22} />Options</button>
      <button classList={{ on: backstage() === "about" }} onClick={() => setBackstage("about")}><Icon n="upd" size={22} />Updates</button>
    </aside>
    <section class="bsmain">
      <Show when={backstage() === "home"}><Home /></Show>
      <Show when={backstage() === "export"}><Export /></Show>
      <Show when={backstage() === "options"}><OptionsPage /></Show>
      <Show when={backstage() === "print"}><PrintPage /></Show>
      <Show when={backstage() === "roadmap"}><RoadmapPage /></Show>
      <Show when={backstage() === "about"}><div class="about"><h1>Updates</h1><UpdateCard /><p><button class="btn" onClick={() => D.shortcuts()}>Keyboard shortcuts</button></p></div></Show>
    </section>
  </div>
);
