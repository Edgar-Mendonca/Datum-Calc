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
  await p.click(".bs aside .back"); await p.waitForTimeout(300);

  // 2. The sheet: title block and every region, in order.
  check("title block shows the calc number", (await p.textContent(".tblock")).includes("CALC-001"));
  const texts = await p.$$eval(".region .rtxt", els => els.map(e => e.textContent));
  check("regions show top to bottom", texts.length === 6 && texts[1] === "Q = 120 m^3/h" && texts[4].startsWith("check"), JSON.stringify(texts));
  check("inputs are highlighted", (await p.$$(".region.s-input")).length === 2);
  check("the page is A4 wide", Math.abs((await p.$eval(".cpage", e => e.getBoundingClientRect().width)) - 210 * 96 / 25.4) < 2);

  // 3. Click selects, double-click edits, Escape saves the text.
  const r2 = await p.$$(".region");
  await r2[2].click(); await p.waitForTimeout(100);
  check("click selects a region", await r2[2].evaluate(e => e.classList.contains("sel")));
  check("Format tab appears for a selected region", (await p.textContent(".ctxgrp")).includes("Region Tools"));
  await r2[2].dblclick(); await p.waitForTimeout(150);
  check("double-click opens the editor", !!(await p.$(".region.editing textarea")));
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
  check("print preview shows the page with its title block", n === 1 && (await p.textContent(".pprev .ptb")).includes("CALC-001") && (await p.textContent(".pprev .pcfoot")).includes("Page 1 of 1"), String(n));
  check("references print at the end under their heading", (await p.textContent(".pprev .pbody")).includes("References"));
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
  const ink = await p.$eval(".rtxt", e => getComputedStyle(e).color);
  check("sheet text stays dark on the page in dark theme", ink === "rgb(27, 39, 51)", ink);

  check("no script errors", errors.length === 0, errors.join(" | "));
  await b.close(); srv.close();
  console.log(failed ? `${failed} failed` : "all passed");
  process.exit(failed ? 1 : 0);
})();
