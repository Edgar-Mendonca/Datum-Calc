// UI tests: the built window (ui/dist) in Chromium with the engine's real output for a small sheet
// (cargo run -p datum-core --example dump > tests_ui/fixture.json) behind a mocked Tauri bridge.
// Run: npm run build:ui && node tests_ui/run.cjs
const { chromium } = require("playwright");
const fs = require("fs"), http = require("http"), path = require("path");
const root = path.join(__dirname, "../ui/dist"), fix = JSON.parse(fs.readFileSync(path.join(__dirname, "fixture.json")));
const types = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".woff": "font/woff", ".ttf": "font/ttf", ".html": "text/html" };
const srv = http.createServer((q, r) => {
  const f = path.join(root, q.url === "/" ? "index.html" : decodeURIComponent(q.url.split("?")[0]));
  fs.readFile(f, (e, b) => { if (e) { r.writeHead(404); return r.end(); } r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" }); r.end(b); });
});
let failed = 0;
const check = (name, ok, info = "") => { console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : "  " + info}`); if (!ok) failed++; };

(async () => {
  await new Promise(ok => srv.listen(0, ok));
  const url = `http://localhost:${srv.address().port}/`;
  const b = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on("pageerror", e => errors.push(e.message));
  await p.addInitScript(f => {
    const opts = { autosave: 0, beta: false, theme: "light", openLast: false, units: "SI", digits: 4, paper: "A4", company: "", by: "", logo: "", number: "auto" };
    const m = { ...f, options: opts, startup_file: null, recovery: null, check_update: null, recent: [], version: "test" };
    window.__calls = [];
    window.__TAURI__ = { core: { invoke: async (c, a) => { window.__calls.push([c, a]); if (c in m) return m[c]; if (c === "apply" || c === "history" || c === "new_sheet") return f.state; return null; } }, event: { listen: async () => () => {} } };
  }, fix);
  const lastApply = () => p.evaluate(() => JSON.stringify(window.__calls.filter(c => c[0] === "apply").pop() ?? null));
  await p.goto(url);
  await p.waitForTimeout(600);

  // 1. The app opens on the File page; its sidebar is the Calc amber.
  const bg = await p.$eval(".bs aside", el => getComputedStyle(el).backgroundColor);
  check("File sidebar is amber", bg === "rgb(168, 101, 15)", bg);
  check("File page offers a blank sheet", (await p.textContent(".tiles")).includes("Blank Sheet"));
  // 1b. File › New lists the ten calc packs by field; a click opens one (0.1.3).
  await p.click(".bs aside button:has-text('New')"); await p.waitForTimeout(200);
  check("New lists the seventeen calc packs", (await p.$$(".tiles.packs .tile")).length === 17 && (await p.$$eval("h3.packf", h => h.map(x => x.textContent))).join() === "Rotating equipment,Process,Mechanical,Electrical");
  await p.click(".tiles.packs .tile:has-text('Pump power')"); await p.waitForTimeout(200);
  check("a pack tile opens that pack", JSON.stringify(await p.evaluate(() => window.__calls.filter(c => c[0] === "new_from_pack").pop())).includes("pump-power"));
  await p.click(".tabs .file").catch(() => {}); await p.waitForTimeout(100);
  await p.click(".bs aside .back"); await p.waitForTimeout(300);

  // 2. The sheet: title block and every region, in order.
  check("title block shows the calc number", (await p.textContent(".tblock")).includes("CALC-001"));
  check("every region shows", (await p.$$(".region")).length === 13);
  check("maths is drawn with KaTeX", (await p.$$(".region .katex")).length >= 9);
  check("the heading shows as text", (await p.textContent(".region.s-heading .rtxt")) === "Pump power and motor size");
  check("inputs are highlighted", (await p.$$(".region.s-input")).length === 4);
  check("the page is A4 wide", Math.abs((await p.$eval(".cpage", e => e.getBoundingClientRect().width)) - 210 * 96 / 25.4) < 2);

  // 3. Click selects, double-click edits, Escape saves the text.
  const r2 = await p.$$(".region");
  await r2[2].click(); await p.waitForTimeout(100);
  check("click selects a region", await r2[2].evaluate(e => e.classList.contains("sel")));
  check("Format tab appears for a selected region", (await p.textContent(".ctxgrp")).includes("Region Tools"));
  await r2[2].dblclick(); await p.waitForTimeout(150);
  check("double-click opens the editor", !!(await p.$(".region.editing textarea")));
  check("the editor holds the typed text", (await p.$eval(".region.editing textarea", t => t.value)) === "H = 45 m");
  await p.keyboard.press("End"); await p.keyboard.type("0"); await p.keyboard.press("Escape"); await p.waitForTimeout(150);
  check("Escape saves the edit", (await lastApply()).includes('"Set"') && (await lastApply()).includes("H = 45 m0"), await lastApply());

  // 4. Keys: arrows select, Ctrl+M inserts a math region below, Alt+↓ moves, Delete deletes.
  await p.click(".cpage", { position: { x: 5, y: 5 } }).catch(() => {});
  await (await p.$$(".region"))[1].click();
  await p.keyboard.press("ArrowDown"); await p.waitForTimeout(80);
  check("↓ selects the next region", await (await p.$$(".region"))[2].evaluate(e => e.classList.contains("sel")));
  await p.keyboard.press("Alt+ArrowDown"); await p.waitForTimeout(100);
  check("Alt+↓ moves the region down", (await lastApply()).includes('"Move"') && (await lastApply()).includes('"up":false'), await lastApply());
  await p.keyboard.press("Delete"); await p.waitForTimeout(100);
  check("Delete removes the region", (await lastApply()).includes('"Delete"'), await lastApply());
  await p.keyboard.press("Control+m"); await p.waitForTimeout(150);
  check("Ctrl+M inserts a math region below the selection", (await lastApply()).includes('"Insert"') && (await lastApply()).includes('"kind":"math"') && (await lastApply()).includes('"at":3'), await lastApply());
  await p.keyboard.press("Escape");

  // 4b. Results, checks, errors, plots, the unit menu and the Variables panel (0.1.1).
  const R = i => p.$$(".region").then(r => r[i]);
  check("a result shows with its unit", (await (await R(5)).textContent()).includes("19.61") && (await (await R(5)).textContent()).includes("kW"));
  check("an input does not repeat its value", (await (await R(1)).$$(".res")).length === 0);
  check("a check shows PASS and the utilisation", (await (await R(7)).textContent()).includes("PASS") && (await (await R(7)).textContent()).includes("0.89"));
  check("the PASS badge is green", (await (await R(7)).$eval(".badge", b => getComputedStyle(b).color)) === "rgb(31, 122, 77)");
  const er = await (await R(8)).$(".rerr");
  check("an error shows on its own line", !!er && (await er.textContent()).includes("Cannot add a power and a flow"));
  check("the wrong part of the line is marked", (await (await R(8)).$eval(".marked mark", m => m.textContent)) === "P + Q");
  check("a plot draws a chart", !!(await (await R(9)).$("canvas")) && (await (await R(9)).$eval("canvas", c => c.width)) > 100);
  await (await R(5)).click(); await p.waitForTimeout(80);
  await (await R(5)).$eval(".unitbtn", b => b.click()); await p.waitForTimeout(120);
  check("the unit menu offers hp", (await p.$$eval(".menu button", bs => bs.map(b => b.textContent))).some(t => t.includes("hp")));
  await p.click(".menu button:has-text('hp')"); await p.waitForTimeout(120);
  check("choosing a unit sets the region's unit", (await lastApply()).includes('"field":"unit"') && (await lastApply()).includes('"hp"'), await lastApply());
  const vars = await p.$$eval(".vars .vlist tr", rs => rs.map(r => r.textContent));
  check("the Variables panel lists every name with its value", vars.length === 9 && vars.some(v => v.includes("19.61") && v.includes("kW")), JSON.stringify(vars));
  check("the panel counts the checks and errors", (await p.textContent(".vars")).includes("1 of 1 pass") && (await p.textContent(".vars")).includes("1 line with errors"));
  await p.click(".vars .vlist tr:nth-child(2)"); await p.waitForTimeout(100);
  check("clicking a variable selects its line", await (await R(2)).evaluate(e => e.classList.contains("sel")));
  check("the status bar counts the checks", (await p.textContent(".status")).includes("1/1 checks pass"));

  // 4c. Tables, pictures, paste and the units list (0.1.2).
  const tb = await R(10);
  check("a table shows its headings, units and worked-out column", (await tb.$$("thead .katex")).length === 3 && (await tb.textContent()).includes("(kW)") && (await tb.$$eval("tbody tr:nth-child(3) td", t => t.map(x => x.textContent))).join("|") === "100|40|10.9");
  await tb.dblclick(); await p.waitForTimeout(150);
  check("double-click opens the table editor", !!(await p.$(".teditor")));
  const c = await p.$('.teditor input[data-r="1"][data-c="1"]'); await c.fill("57"); await p.keyboard.press("Enter"); await p.waitForTimeout(80);
  check("Enter moves down a row in the table", await p.evaluate(() => document.activeElement?.dataset.r === "2"));
  await p.keyboard.press("Enter"); await p.waitForTimeout(80);
  check("Enter on the last row adds a row", (await p.$$(".teditor tbody tr")).length === 4);
  await p.click(".teditor .btn.pri"); await p.waitForTimeout(150);
  const ta = await lastApply();
  check("Done saves the table as one change", ta.includes('"Batch"') && ta.includes('"57"') && ta.includes('"field":"data"'), ta);
  check("a picture shows with its caption", !!(await (await R(11)).$("img")) && (await (await R(11)).textContent()).includes("Free-body diagram"));
  await (await R(11)).click(); await p.waitForTimeout(80);
  await p.click(".ctxgrp button"); await p.waitForTimeout(80);
  await p.click(".ribbon button:has-text('75 %')"); await p.waitForTimeout(100);
  check("Format › Picture sets its size", (await lastApply()).includes('"width":75'), await lastApply());
  await p.evaluate(() => { const dt = new DataTransfer(); dt.setData("text/plain", "a = 2 m\ncheck a < 3 m"); document.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true })); }); await p.waitForTimeout(150);
  const pa = await lastApply();
  check("pasting text adds one region per line", pa.includes('"a = 2 m"') && pa.includes('"kind":"check"'), pa);
  await p.evaluate(async () => { const dt = new DataTransfer(); const b = await (await fetch("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==")).blob(); dt.items.add(new File([b], "p.png", { type: "image/png" })); document.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true })); }); await p.waitForTimeout(300);
  check("pasting a picture adds a picture region", (await lastApply()).includes('"kind":"image"') && (await lastApply()).includes("data:image/png"), (await lastApply()).slice(0, 120));
  await p.click(".tabs > button:text-is('Units')"); await p.waitForTimeout(80);
  await p.evaluate(() => { window.__TAURI__.core.invoke = ((orig) => async (c, a) => c === "lists" ? { units: [["kW", "kilowatt", "Power", "m²·kg/s³"], ["hp", "horsepower (mechanical)", "Power", "m²·kg/s³"], ["psi", "pound-force per square inch", "Pressure", "kg/(m·s²)"]], constants: [["g", "standard gravity"]], functions: [["sqrt(x)", "square root"]] } : orig(c, a))(window.__TAURI__.core.invoke); });
  await p.click(".ribbon button:has-text('All Units')"); await p.waitForTimeout(200);
  check("the units list groups units by kind", (await p.$$eval(".ulist h4", h => h.map(x => x.textContent))).join() === "Power,Pressure");
  await p.fill(".ulhead .find", "psi"); await p.waitForTimeout(80);
  check("the units list searches", (await p.$$(".ulist tr")).length === 1);
  await p.keyboard.press("Escape");
  await p.keyboard.press("Escape");

  // 5. Ribbon: the seven tabs, symbols type into the editor, no sideways scrolling.
  const tabs = await p.$$eval(".tabs > button:not(.file)", bs => bs.map(b => b.textContent));
  check("ribbon tabs", JSON.stringify(tabs) === JSON.stringify(["Home", "Insert", "Math", "Units", "Check", "View", "Help"]), JSON.stringify(tabs));
  await (await p.$$(".region"))[3].dblclick(); await p.waitForTimeout(120);
  await p.click(".tabs > button:text-is('Math')"); await p.waitForTimeout(100);
  await p.click(".rb.key:text-is('σ')"); await p.waitForTimeout(80);
  check("a Greek key types its name into the editor", (await p.$eval(".region.editing textarea", t => t.value)).endsWith("sigma"));
  await p.keyboard.press("Escape"); await p.waitForTimeout(100);
  for (const t of tabs) {
    await p.click(`.tabs > button:text-is('${t}')`); await p.waitForTimeout(60);
    const over = await p.$eval(".ribbon", r => r.scrollWidth - r.clientWidth);
    check(`${t} tab fits without scrolling`, over <= 1, String(over));
  }
  await p.setViewportSize({ width: 960, height: 700 }); await p.waitForTimeout(200);
  await p.click(".tabs > button:text-is('Math')"); await p.waitForTimeout(150);
  check("Math tab fits at 960 px", (await p.$eval(".ribbon", r => r.scrollWidth - r.clientWidth)) <= 1);
  await p.setViewportSize({ width: 1440, height: 900 });

  // 5b. Right-click a region: Copy Result; Export › CSV holds the inputs, results and checks (0.1.3).
  await p.evaluate(() => { window.__clip = ""; navigator.clipboard.writeText = async t => { window.__clip = t; }; });
  await (await R(5)).click({ button: "right" }); await p.waitForTimeout(120);
  await p.click(".menu button:has-text('Copy Result')"); await p.waitForTimeout(120);
  check("Copy Result copies the value with its unit", (await p.evaluate(() => window.__clip)) === "19.61 kW", await p.evaluate(() => window.__clip));
  await p.click(".tabs .file"); await p.waitForTimeout(120);
  await p.click(".bs aside button:has-text('Export')"); await p.waitForTimeout(150);
  await p.click(".tile:has-text('Inputs and results')"); await p.waitForTimeout(200);
  const csv = await p.evaluate(() => window.__calls.filter(c => c[0] === "save_file").pop()?.[1]);
  check("CSV export lists inputs, results and checks", !!csv && csv.name.endsWith(".csv") && csv.data.startsWith("Name,Value,Unit,Kind") && csv.data.includes("Q,120,m³/h,input") && csv.data.includes("P,19.61,kW,result") && csv.data.includes("PASS"), JSON.stringify(csv).slice(0, 300));
  await p.click(".bs aside .back"); await p.waitForTimeout(150);

  // 5c. View › Numbers saves the setting with the sheet; the side panel lists every check (0.1.5).
  await p.click(".tabs > button:text-is('View')"); await p.waitForTimeout(80);
  await p.click(".ribbon button:has-text('Numbers')"); await p.waitForTimeout(120);
  check("View › Numbers turns on the substituted line", (await lastApply()).includes('"field":"subst"') && (await lastApply()).includes('"value":true'), await lastApply());
  check("the side panel lists the checks with their badges", (await p.$$(".vars .vchecks tr")).length === 1 && (await p.textContent(".vars .vchecks")).includes("PASS"));
  await p.click(".vars .vchecks tr"); await p.waitForTimeout(100);
  check("clicking a listed check selects it", await (await R(7)).evaluate(e => e.classList.contains("sel")));
  const badgeW = await p.$eval(".vars .vchecks .badge", b => b.scrollWidth <= b.clientWidth + 1);
  check("the check badge in the panel is not cut", badgeW);

  // 5d. Figures per sheet, Duplicate, Insert Above (0.1.6).
  await p.click(".tabs > button:text-is('View')"); await p.waitForTimeout(80);
  await p.click(".figs button:text-is('5')"); await p.waitForTimeout(100);
  check("View › Figures sets the sheet's significant figures", (await lastApply()).includes('"field":"digits"') && (await lastApply()).includes('"value":5'), await lastApply());
  await (await R(5)).click(); await p.keyboard.press("Control+d"); await p.waitForTimeout(120);
  const dup = await lastApply();
  check("Ctrl+D duplicates the region below it with its style and unit", dup.includes('"Batch"') && dup.includes('"at":6') && dup.includes("rho*g*Q*H/eta") && dup.includes('"field":"unit"'), dup);
  await (await R(5)).click(); await p.keyboard.press("Control+Shift+m"); await p.waitForTimeout(120);
  check("Ctrl+Shift+M inserts a math region above", (await lastApply()).includes('"Insert"') && (await lastApply()).includes('"at":5'), await lastApply());
  await p.keyboard.press("Escape"); await p.waitForTimeout(80);

  // 6. ? help opens a card.
  await p.click(".tabs > button:text-is('Home')");
  await p.click(".grp .cap .help"); await p.waitForTimeout(120);
  check("? opens a help card", (await p.textContent(".dlg .helpcard")).length > 20);
  await p.keyboard.press("Escape");

  // 7. Title block dialog sends one edit per changed field.
  await p.click(".tblock"); await p.waitForTimeout(150);
  const inputs = await p.$$(".dlg .form input");
  await inputs[3].fill("E. Engineer"); await p.click(".dlg .btn.pri"); await p.waitForTimeout(150);
  check("title block edit", (await lastApply()).includes('"field":"by"') && (await lastApply()).includes("E. Engineer"), await lastApply());

  // 8. Options: each label sits left of its control.
  await p.click(".tabs .file"); await p.waitForTimeout(150);
  await p.click(".bs aside button:has-text('Options')"); await p.waitForTimeout(200);
  const bad = await p.$$eval(".opt", os => os.filter(o => o.children[0].getBoundingClientRect().right > o.children[1].getBoundingClientRect().left + 1).length);
  check("Options labels sit left of their controls", bad === 0 && (await p.$$(".opt")).length >= 8, String(bad));

  // 9. Print: a preview page with the title block; printing lays out the same pages at A4.
  await p.click(".bs aside button:has-text('Print')"); await p.waitForTimeout(500);
  const n = (await p.$$(".pprev .ppage")).length;
  check("print preview shows the pages with the title block on each", n >= 2 && (await p.$$(".pprev .ptb")).length === n && (await p.textContent(".pprev .ptb")).includes("CALC-001") && (await p.textContent(".pprev .pcfoot")).includes(`Page 1 of ${n}`), String(n));
  check("references print at the end under their heading", (await p.$$eval(".pprev .ppage", ps => ps[ps.length - 1].textContent)).includes("References"));
  check("no region is cut at a page edge", await p.$$eval(".pprev .pbody", bs => bs.every(b => b.scrollHeight <= b.clientHeight + 1)));
  check("the printed sheet shows the maths and the PASS badge", (await p.$$(".pprev .katex")).length >= 9 && (await p.textContent(".pprev")).includes("PASS"));
  await p.click(".pset label.chk:has-text('Summary of checks')"); await p.waitForTimeout(150);
  check("Print › Summary of checks is saved with the sheet", (await lastApply()).includes('"Ui"') && (await lastApply()).includes('"summary":true'), await lastApply());
  check("the printed sheet hides the unit buttons", (await p.$$(".pprev .unitbtn, .pprev .pngbtn")).length === 0);
  await p.evaluate(() => { window.print = () => { window.__printed = [document.querySelectorAll("#printroot .ppage").length, [...document.querySelectorAll("style")].map(s => s.textContent).find(t => t.includes("@page")) || ""]; }; });
  await p.click(".pbtns .btn"); await p.waitForTimeout(500);
  const pr = await p.evaluate(() => window.__printed);
  check("Print renders every page at the paper size", pr && pr[0] === n && /size: 210mm 297mm/.test(pr[1]), JSON.stringify(pr));
  await p.click(".bs aside .back"); await p.waitForTimeout(150);

  // 10. Tell Me finds commands.
  await p.keyboard.press("Control+k"); await p.waitForTimeout(120);
  await p.keyboard.type("title"); await p.waitForTimeout(80);
  check("Tell Me finds the title block", (await p.textContent(".pal .pl")).includes("Title block"));
  await p.keyboard.press("Escape");

  // 11. Dark theme keeps the page white-ish and the text readable.
  await p.evaluate(() => { document.documentElement.dataset.theme = "dark"; }); await p.waitForTimeout(100);
  const ink = await p.$eval(".region .rtxt", e => getComputedStyle(e).color);
  check("sheet text stays dark on the page in dark theme", ink === "rgb(27, 39, 51)", ink);

  // 12. A region that cannot be drawn shows a note; the rest of the window still works (0.1.6).
  const p2 = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const broken = JSON.parse(JSON.stringify(fix.state));
  broken.calc.regions[7].check.ratio = "not a number";
  broken.calc.regions[9].plot = { xs: null, series: null };
  broken.sheet.ui = { print: { summary: true } };
  await p2.addInitScript(st => { window.__TAURI__ = { core: { invoke: async c => c === "state" ? st : c === "options" ? { theme: "light", autosave: 0, units: "SI", digits: 4, paper: "A4", number: "auto" } : c === "recent" || c === "packs" ? [] : null }, event: { listen: async () => () => {} } }; }, broken);
  await p2.goto(url); await p2.waitForTimeout(500); await p2.click(".bs aside .back"); await p2.waitForTimeout(400);
  check("a region that cannot be drawn shows a note instead of blanking the window", (await p2.$$(".region")).length === 13 && (await p2.textContent(".cpage")).includes("could not be shown") && (await p2.$$(".region .katex")).length > 8);
  await p2.keyboard.press("Control+p"); await p2.waitForTimeout(500);
  check("the print summary lists the checks first", (await p2.textContent(".pprev .psum")).includes("Summary of checks"));
  await p2.close();

  // 13. Complex values in polar form, with their numbers, draw without a KaTeX error; View › Polar turns it off (0.1.8).
  const p3 = await b.newPage({ viewport: { width: 1440, height: 900 } });
  p3.on("pageerror", e => errors.push(String(e)));
  await p3.addInitScript(st => { window.__calls = []; window.__TAURI__ = { core: { invoke: async (c, a) => { window.__calls.push([c, a]); return c === "state" || c === "apply" ? st : c === "options" ? { theme: "light", autosave: 0, units: "SI", digits: 4, paper: "A4", number: "auto" } : c === "recent" || c === "packs" ? [] : null; } }, event: { listen: async () => () => {} } }; }, fix.ac);
  await p3.goto(url); await p3.waitForTimeout(500); await p3.click(".bs aside .back"); await p3.waitForTimeout(400);
  const acText = await p3.textContent(".cpage");
  check("complex results draw in polar form", acText.includes("∠") && (await p3.$$(".cpage .katex-error")).length === 0 && (await p3.$$(".cpage .region .katex")).length > 20, String((await p3.$$(".cpage .katex-error")).length));
  check("the AC pack passes both checks", (await p3.$$(".cpage .badge.pass")).length === 2, String((await p3.$$(".cpage .badge.pass")).length));
  await p3.click(".tabs > button:text-is('View')"); await p3.waitForTimeout(100);
  await p3.click(".ribbon button:has-text('Polar')"); await p3.waitForTimeout(120);
  const acApply = await p3.evaluate(() => JSON.stringify(window.__calls.filter(c => c[0] === "apply").pop() ?? null));
  check("View › Polar turns polar form off", acApply.includes('"field":"polar"') && acApply.includes('"value":false'), acApply);
  await p3.close();

  check("no script errors", errors.length === 0, errors.join(" | "));
  await b.close(); srv.close();
  console.log(failed ? `${failed} failed` : "all passed");
  process.exit(failed ? 1 : 0);
})();
