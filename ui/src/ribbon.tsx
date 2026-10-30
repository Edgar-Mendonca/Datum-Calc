// Title bar, quick access toolbar, Tell Me and the ribbon.
import { For, Show, JSX, createSignal, createMemo, onMount } from "solid-js";
import { out, tab, setTab, setBackstage, openMenu, openDialog, closeDialog, theme, setTheme, selId, zoom, setZoom, act, varsPanel } from "./store";
import { C, ALL_CMDS, typeText } from "./cmds";
import * as D from "./dialogs";
import { Icon } from "./ui";
import { Help, helpCard, HELP } from "./help";
import { pickPicture } from "./sheet";

const B = (p: { i: string; l: string; run?: (e: MouseEvent) => void; big?: boolean; on?: boolean; off?: boolean; dd?: (e: MouseEvent) => void; title?: string }) => p.big && p.run && p.dd ? (
  // Split button: the picture runs the command, the label opens the menu.
  <div class="rbw big splitbig" classList={{ on: p.on }}>
    <button class="rb top" title={p.title ?? p.l} onClick={e => p.run!(e)}><Icon n={p.i} /></button>
    <button class="rb bot" title={`${p.l} options`} onClick={e => p.dd!(e)}><span>{p.l}</span><b class="car">▾</b></button>
  </div>
) : (
  <div class="rbw" classList={{ big: p.big }}>
    <button class="rb" classList={{ big: p.big, on: p.on }} disabled={p.off} title={p.title ?? p.l} onClick={e => (p.run ?? p.dd)?.(e)}><Icon n={p.i} />{p.big ? <span>{p.l}{p.dd && !p.run ? " ▾" : ""}</span> : <span>{p.l}</span>}{!p.big && p.dd && !p.run && <b class="car">▾</b>}</button>
    {p.run && p.dd && <button class="rb split" onClick={e => p.dd!(e)}>▾</button>}
  </div>
);
const G = (p: { l: string; h?: string; children: JSX.Element }) => <div class="grp"><div class="items">{p.children}</div><div class="cap">{p.l}{p.h && <Help k={p.h} />}</div></div>;
const S_ = (p: { children: JSX.Element }) => <div class="stack">{p.children}</div>;
/** A small key that types text into the region being edited. */
const K = (p: { t: string; ins?: string; back?: number; title?: string }) => <button class="rb key" title={p.title ?? `Type ${p.ins ?? p.t}`} onMouseDown={e => e.preventDefault()} onClick={() => typeText(p.ins ?? p.t, p.back)}>{p.t}</button>;

const GREEK: [string, string][] = [["α", "alpha"], ["β", "beta"], ["γ", "gamma"], ["δ", "delta"], ["Δ", "Delta"], ["ε", "epsilon"], ["η", "eta"], ["θ", "theta"], ["λ", "lambda"], ["μ", "mu"], ["ν", "nu"], ["π", "pi"], ["ρ", "rho"], ["σ", "sigma"], ["τ", "tau"], ["φ", "phi"], ["ω", "omega"], ["Ω", "Omega"]];
const FUNCS: [string, string][] = [["sqrt", "sqrt()"], ["sin", "sin()"], ["cos", "cos()"], ["tan", "tan()"], ["ln", "ln()"], ["log", "log()"], ["exp", "exp()"], ["abs", "abs()"], ["min", "min(, )"], ["max", "max(, )"], ["round", "round(, 2)"], ["if", "if(, , )"]];
const UNITS: [string, string[]][] = [
  ["Length", ["m", "mm", "km", "in", "ft"]], ["Mass", ["kg", "g", "t", "lb"]], ["Force", ["N", "kN", "MN", "lbf", "kip"]], ["Pressure", ["Pa", "kPa", "MPa", "bar", "psi"]],
  ["Energy", ["J", "kJ", "kWh", "BTU"]], ["Power", ["W", "kW", "MW", "hp"]], ["Flow", ["m^3/h", "L/s", "gpm"]], ["Speed", ["m/s", "rpm", "rad/s"]], ["Temperature", ["°C", "K", "°F", "delta°C"]],
];
const unitMenu = () => UNITS.map(([l, us]) => ({ label: l, sub: us.map(u => ({ label: u, run: () => typeText(` ${u}`) })) }));

/** Tell Me / command search (Ctrl+K, Alt+Q). */
export const palette = () => openDialog(() => {
  const [q, setQ] = createSignal(""), [k, setK] = createSignal(0);
  const list = createMemo(() => { const s = q().toLowerCase(); return ALL_CMDS.filter(c => c[0].toLowerCase().includes(s)).slice(0, 14); });
  const go = (i: number) => { const c = list()[i]; if (c) { closeDialog(); setTimeout(() => c[1]()); } };
  let input!: HTMLInputElement;
  onMount(() => input.focus());
  return (
    <div class="ov top" onMouseDown={e => e.target === e.currentTarget && closeDialog()}>
      <div class="pal">
        <input ref={input} placeholder="Tell me what you want to do" value={q()} onInput={e => { setQ(e.currentTarget.value); setK(0); }}
          onKeyDown={e => { if (e.key === "ArrowDown") { e.preventDefault(); setK(Math.min(list().length - 1, k() + 1)); } else if (e.key === "ArrowUp") { e.preventDefault(); setK(Math.max(0, k() - 1)); } else if (e.key === "Enter") { e.preventDefault(); go(k()); } else if (e.key === "Escape") closeDialog(); }} />
        <div class="pl"><For each={list()}>{(c, i) => <button classList={{ on: i() === k() }} onMouseEnter={() => setK(i())} onClick={() => go(i())}><span>{c[0]}</span>{c[2] && <kbd>{c[2]}</kbd>}</button>}</For></div>
      </div>
    </div>
  );
});

export const TitleBar = (p: { update: string | null }) => (
  <header class="top">
    <div class="bar">
      <div class="qat">
        <img src="logo.svg" alt="" class="logo" />
        <button title="Save (Ctrl+S)" onClick={() => C.save()}><Icon n="save" /></button>
        <button title="Undo (Ctrl+Z)" disabled={!out()?.undo} onClick={C.undo}><Icon n="undo" /></button>
        <button title="Redo (Ctrl+Y)" disabled={!out()?.redo} onClick={C.redo}><Icon n="redo" /></button>
        <button title="Print (Ctrl+P)" onClick={C.print}><Icon n="print" /></button>
      </div>
      <div class="title">{out() ? `${out()!.sheet.name}${out()!.dirty ? " •" : ""} — Datum Calc` : "Datum Calc"}</div>
      <button class="search" onClick={palette} title="Tell me what you want to do (Alt+Q)"><Icon n="search" /><span>Tell me what you want to do</span><kbd>Ctrl+K</kbd></button>
      <Show when={p.update}><button class="upd" onClick={() => setBackstage("about")}>Update {p.update} available</button></Show>
      <button class="theme" title="Theme: Auto follows Windows" onClick={() => setTheme(theme() === "auto" ? "light" : theme() === "light" ? "dark" : "auto")}>Theme · {theme()}</button>
    </div>
    <nav class="tabs">
      <button class="file" onClick={() => setBackstage("home")}>File</button>
      <For each={[["home", "Home"], ["insert", "Insert"], ["math", "Math"], ["units", "Units"], ["check", "Check"], ["view", "View"], ["help", "Help"]]}>{([k, l]) =>
        <button classList={{ on: tab() === k }} onClick={() => setTab(k)}>{l}</button>}</For>
      <Show when={selId() > 0}><div class="ctxgrp"><small>Region Tools</small><button classList={{ on: tab() === "format" }} onClick={() => setTab("format")}>Format</button></div></Show>
    </nav>
  </header>
);

/** Shrinks the ribbon in steps (labels of small buttons, then of big ones) instead of scrolling sideways. */
const fitRibbon = (el: HTMLElement) => {
  const fit = () => {
    const gs = [...el.querySelectorAll<HTMLElement>(":scope > .grp")];
    el.dataset.fit = "0"; gs.forEach(g => delete g.dataset.fit);
    const over = () => el.scrollWidth > el.clientWidth + 1;
    if (!over()) return;
    el.dataset.fit = "1";
    for (const lvl of ["2", "3"]) for (let i = gs.length - 1; i >= 0 && over(); i--) gs[i].dataset.fit = lvl;
  };
  new ResizeObserver(fit).observe(el);
  new MutationObserver(() => requestAnimationFrame(fit)).observe(el, { childList: true, subtree: false });
};

const STYLES: [string, string][] = [["", "Normal"], ["input", "Input (highlighted)"], ["heading", "Heading"], ["ref", "Reference"], ["note", "Note"]];
const sel = () => out()?.sheet.regions.find(r => r.id === selId());
const setStyle = (s: string) => { const r = sel(); if (r) act({ Set: { id: r.id, field: "style", value: s } }); };

export const Ribbon = () => (
  <section class="ribbon" ref={fitRibbon}>
    <Show when={tab() === "home"}>
      <G l="Clipboard" h="clipboard"><B big i="paste" l="Paste" run={() => C.paste()} /><S_><B i="copy" l="Copy" run={C.copy} /><B i="cut" l="Cut" run={() => C.copy().then(C.del)} /><B i="copy" l="Copy Result" run={C.copyResult} /></S_></G>
      <G l="Regions" h="regions"><B big i="math" l="Math" title="New math region (Ctrl+M)" run={() => C.insert("math")} /><B big i="text" l="Text" title="New text region (Ctrl+T)" run={() => C.insert("text")} /><B big i="check" l="Check" run={() => C.insert("check")} /></G>
      <G l="Arrange" h="arrange"><S_><B i="up" l="Move Up" run={() => C.move(true)} /><B i="down" l="Move Down" run={() => C.move(false)} /><B i="del" l="Delete" run={C.del} /></S_><S_><B i="copy" l="Duplicate" title="Duplicate the region (Ctrl+D)" run={C.duplicate} /><B i="math" l="Insert Above" title="New math region above (Ctrl+Shift+M)" run={C.insertAbove} /></S_></G>
      <G l="Sheet" h="sheetset"><B big i="title" l="Title Block" run={() => D.titleBlock()} /><S_><B i="units" l="SI units" on={out()?.sheet.units === "SI"} run={() => C.units("SI")} /><B i="units" l="US units" on={out()?.sheet.units === "US"} run={() => C.units("US")} /></S_></G>
      <G l="Print" h="print"><B big i="print" l="Print / PDF" run={C.print} /></G>
    </Show>
    <Show when={tab() === "insert"}>
      <G l="Regions" h="insert"><B big i="math" l="Math" run={() => C.insert("math")} /><B big i="text" l="Text" run={() => C.insert("text")} /><B big i="check" l="Check" run={() => C.insert("check")} /><B big i="solve" l="Solve" run={() => C.insert("solve")} /><B big i="plot" l="Plot" run={() => C.insert("plot")} /><B big i="table" l="Table" run={() => C.insert("table")} /><B big i="image" l="Picture" run={() => C.picture()} />
        <S_><B i="text" l="Heading" run={() => C.insert("text").then(() => setStyle("heading"))} /><B i="text" l="Reference" run={() => C.insert("text", "Ref: ").then(() => setStyle("ref"))} /><B i="math" l="Input" run={() => C.insert("math").then(() => setStyle("input"))} /></S_></G>
    </Show>
    <Show when={tab() === "math"}>
      <G l="Operators" h="symbols"><div class="keys2"><K t="+" ins=" + " /><K t="−" ins=" - " /><K t="×" ins="*" /><K t="÷" ins="/" /><K t="xⁿ" ins="^" /><K t="√" ins="sqrt()" back={1} /><K t="( )" ins="()" back={1} /><K t="=" ins=" = " /><K t="≤" ins=" <= " /><K t="≥" ins=" >= " /><K t="x₁" ins="_" title="Subscript: type an underscore, then the subscript" /><K t=".." ins=" .. " title="A range: x = 0, 0.1 .. 10" /></div></G>
      <G l="Greek" h="symbols"><div class="keys2"><For each={GREEK}>{([g, n]) => <K t={g} ins={n} title={`${n} (type “${n}”)`} />}</For></div></G>
      <G l="Functions" h="functions"><div class="keys2 wide"><For each={FUNCS}>{([n, ins]) => <K t={n} ins={ins} back={ins.length - ins.indexOf("(") - 1} />}</For></div><B big i="list" l="All Functions" run={() => D.unitsList("functions")} /></G>
    </Show>
    <Show when={tab() === "units"}>
      <G l="Unit System" h="units"><B big i="units" l="SI" on={out()?.sheet.units === "SI"} run={() => C.units("SI")} /><B big i="units" l="US" on={out()?.sheet.units === "US"} run={() => C.units("US")} /></G>
      <G l="Insert Unit" h="units"><B big i="ruler" l="Unit" dd={e => openMenu(e, unitMenu())} /><B big i="list" l="All Units" run={() => D.unitsList()} /><div class="keys2"><For each={["m", "mm", "kg", "s", "N", "kN", "Pa", "MPa", "bar", "W", "kW", "°C", "K"]}>{u => <K t={u} ins={` ${u}`} />}</For></div></G>
    </Show>
    <Show when={tab() === "check"}>
      <G l="Checks" h="checks"><B big i="check" l="New Check" run={() => C.insert("check")} /><div class="keys2"><K t="≤" ins=" <= " /><K t="<" ins=" < " /><K t="≥" ins=" >= " /><K t=">" ins=" > " /></div></G>
    </Show>
    <Show when={tab() === "view"}>
      <G l="Zoom" h="zoom"><B big i="zin" l="Zoom In" run={C.zoomIn} /><B big i="zout" l="Zoom Out" run={C.zoomOut} /><S_><B i="fit" l="100 %" run={() => setZoom(100)} /><span class="muted zoomv">{zoom()} %</span></S_></G>
      <G l="Figures" h="figures"><div class="keys2 figs"><For each={[3, 4, 5, 6]}>{n => <button class="rb key" classList={{ on: out()?.sheet.digits === n }} title={`${n} significant figures`} onClick={() => C.figures(n)}>{n}</button>}</For></div></G>
      <G l="Show" h="show"><B big i="vars" l="Variables" on={varsPanel()} run={C.vars} /><B big i="math" l="Numbers" title="Show each equation again with its numbers put in (also printed)" on={!!out()?.sheet.subst} run={() => act({ SheetSet: { field: "subst", value: !out()?.sheet.subst } })} /><B big i="polar" l="Polar" title="Show complex values as magnitude and angle (5∠53.13°) instead of 3 + 4j" on={!!out()?.sheet.polar} run={() => act({ SheetSet: { field: "polar", value: !out()?.sheet.polar } })} /></G>
      <G l="Theme" h="theme"><S_><For each={[["auto", "Auto"], ["light", "Light"], ["dark", "Dark"]]}>{([k, l]) => <B i="theme" l={l} on={theme() === k} run={() => setTheme(k)} />}</For></S_></G>
    </Show>
    <Show when={tab() === "format" && sel()}>
      <G l="Region Style" h="regions"><S_><For each={STYLES.slice(0, 3)}>{([k, l]) => <B i="style" l={l} on={(sel()?.style ?? "") === k} run={() => setStyle(k)} />}</For></S_><S_><For each={STYLES.slice(3)}>{([k, l]) => <B i="style" l={l} on={(sel()?.style ?? "") === k} run={() => setStyle(k)} />}</For></S_></G>
      <Show when={sel()?.kind === "image"}><G l="Picture" h="regions"><B big i="image" l="Change" run={() => pickPicture(selId())} /><S_><For each={[25, 50, 75, 100]}>{w => <B i="fit" l={`${w} %`} on={(sel()?.data?.width ?? 60) === w} run={() => act({ Set: { id: selId(), field: "data", value: { ...(sel()?.data ?? {}), width: w } } })} />}</For></S_></G></Show>
      <G l="Kind" h="regions"><S_><For each={[["math", "Math"], ["text", "Text"], ["check", "Check"], ["solve", "Solve"], ["plot", "Plot"]] as const}>{([k, l]) => <B i={k} l={l} on={sel()?.kind === k} run={() => act({ Set: { id: selId(), field: "kind", value: k } })} />}</For></S_></G>
      <G l="Arrange" h="arrange"><S_><B i="up" l="Move Up" run={() => C.move(true)} /><B i="down" l="Move Down" run={() => C.move(false)} /><B i="del" l="Delete" run={C.del} /></S_></G>
    </Show>
    <Show when={tab() === "help"}>
      <G l="Learn"><B big i="report" l="Getting Started" title="Open the Getting Started sheet; the ▾ shows a short summary" run={() => C.pack("getting-started")} dd={e => openMenu(e, [{ label: "Open the Getting Started sheet", run: () => C.pack("getting-started") }, { label: "Summary…", run: () => D.gettingStarted() }])} /><B big i="info" l="Glossary" title="What units, checks, utilisation and other terms mean" run={() => D.glossary()} /><B big i="help" l="What Is This Tool?" title="Every tool explained: click a ? beside a tool, or pick one here" dd={e => openMenu(e, Object.entries(HELP).map(([k, h]) => ({ label: h.t, run: () => helpCard(k) })))} /><B big i="keys" l="Shortcuts" run={() => D.shortcuts()} /><B big i="search" l="Tell Me" run={palette} /></G>
      <G l="Datum Calc on GitHub"><B big i="github" l="GitHub" title="The project and its code" run={() => C.github()} /><B big i="tag" l="Releases" title="Every version and its change log" run={() => C.github("/releases")} /><B big i="bug" l="Report an Issue" run={() => C.github("/issues/new")} /></G>
      <G l="Updates"><B big i="upd" l="Check for Updates" run={() => setBackstage("about")} /></G>
    </Show>
  </section>
);
