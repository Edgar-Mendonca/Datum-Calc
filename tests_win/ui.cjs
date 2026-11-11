// Drives the installed app's real window on Windows (WebView2, opened with a debugging port by smoke.ps1).
// node tests_win/ui.cjs <first|recover> <version>
const { chromium } = require("playwright");
const [mode, version] = process.argv.slice(2);
let failed = 0;
const check = (name, ok, info = "") => { console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : `  ${info}`}`); if (!ok) failed++; };
const wait = ms => new Promise(r => setTimeout(r, ms));

(async () => {
  const b = await chromium.connectOverCDP("http://127.0.0.1:9222");
  const pages = b.contexts().flatMap(c => c.pages());
  const p = pages.find(x => !x.url().startsWith("devtools")) ?? pages[0];
  const errors = [];
  p.on("pageerror", e => errors.push(String(e)));
  await p.waitForSelector(".app", { timeout: 60000 });
  await wait(1500);
  const invoke = (cmd, args) => p.evaluate(([c, a]) => window.__TAURI__.core.invoke(c, a), [cmd, args]);

  if (mode === "first") {
    // The sheet double-clicked in Explorer (passed on the command line) opens at start.
    const v = await invoke("version");
    check("the installed version is the one built", v === version, v);
    await p.waitForSelector(".cpage .region", { timeout: 30000 });
    const text = await p.textContent(".cpage");
    check("a sheet passed on the command line opens", text.includes("AC circuit"), text.slice(0, 120));
    check("its maths is drawn without errors", (await p.$$(".cpage .katex")).length > 20 && (await p.$$(".cpage .katex-error")).length === 0);
    check("the engine works it out: both checks pass", (await p.$$(".cpage .badge.pass")).length === 2, String((await p.$$(".cpage .badge.pass")).length));
    check("complex results show", text.includes("12.04 − 3.346j"));
    // An edit makes the sheet unsaved; a recovery copy is written (smoke.ps1 then kills the window, as a crash).
    const regions = await p.$$(".cpage .region");
    await regions[regions.length - 1].click(); await wait(200);
    await p.keyboard.press("Control+m"); await wait(300);
    await p.keyboard.type("x_test = 42 m"); await p.keyboard.press("Escape"); await wait(500);
    check("an edit is worked out", (await p.textContent(".cpage")).includes("42 m"));
    check("the title shows unsaved changes", (await p.textContent(".top")).includes("•"));
    // New asks first while there are unsaved changes; Cancel keeps the sheet.
    await p.keyboard.press("Control+n"); await wait(400);
    await p.click(".tiles .tile:has-text('Blank Sheet')"); await wait(400);
    check("New asks to save unsaved changes", ((await p.textContent(".dlg").catch(() => "")) ?? "").includes("Save changes?"));
    await p.click(".dlg footer .btn:text-is('Cancel')"); await wait(300);
    // Print preview lays out pages with the title block.
    await p.click(".bs aside button:has-text('Print')"); await wait(800);
    check("print preview shows pages with the title block", (await p.$$(".pprev .ppage")).length >= 1 && (await p.$$(".pprev .ptb")).length >= 1);
    await invoke("autosave"); await wait(500);
  } else if (mode === "recover") {
    // After a crash, the copy is offered; Discard removes it. Then a pack opens and works out.
    await p.waitForSelector(".dlg", { timeout: 30000 });
    const dlg = await p.textContent(".dlg");
    check("after a crash, unsaved work is offered back", dlg.includes("Recover unsaved work?") && dlg.includes("AC circuit"), dlg);
    await p.click(".dlg footer .btn:not(.pri)"); await wait(800);
    await p.click(".bs aside button:has-text('New')"); await wait(400);
    await p.click(".tiles.packs .tile:has-text('RC low-pass')"); await wait(1500);
    const text = await p.textContent(".cpage");
    check("a calc pack opens and works out", text.includes("994.7 Hz") && (await p.$$(".cpage canvas")).length === 2 && (await p.$$(".cpage .katex-error")).length === 0, text.slice(0, 200));
  }
  check("no script errors in the window", errors.length === 0, errors.join(" | "));
  console.log(failed ? `${failed} failed` : "all passed");
  await b.close().catch(() => {});
  process.exit(failed ? 1 : 0);
})().catch(e => { console.log("FAIL", e.message); process.exit(1); });
