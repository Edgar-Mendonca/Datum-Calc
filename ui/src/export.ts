// Export (0.2.10): the calc sheet as a web page (HTML with MathML, which browsers draw without fonts or scripts)
// and as a Word document (.docx, written by the engine with equations Word can edit). Regions hidden from print are
// left out of both. The window gives the drawn plots as pictures.
import katex from "katex";
import { Region } from "./api";
import { S, out, call, toast } from "./store";

const ml = (tex: string) => katex.renderToString(tex, { output: "mathml", throwOnError: false, strict: false, displayMode: false });
const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const ops: Record<string, string> = { "<=": "≤", ">=": "≥", "!=": "≠", "==": "=" };

/** The plots as drawn on the sheet: region id → PNG data URL, with the canvas size. */
const plots = () => {
  const m = new Map<number, { src: string; w: number; h: number }>();
  document.querySelectorAll<HTMLCanvasElement>(".cpage .region canvas").forEach(cv => {
    const id = +((cv.closest(".region") as HTMLElement | null)?.dataset.id ?? 0);
    if (id && cv.width) m.set(id, { src: cv.toDataURL("image/png"), w: cv.width, h: cv.height });
  });
  return m;
};

const fileName = (ext: string) => `${(S().title.title || S().name || "calc").replace(/[\\/:*?"<>|]+/g, " ").trim() || "calc"}.${ext}`;

/** One region as HTML. */
const region = (r: Region, pics: Map<number, { src: string }>): string => {
  const o = out()?.calc.regions.find(x => x.id === r.id);
  if (r.kind === "text") {
    const tag = r.style === "heading" ? "h2" : r.style === "subheading" ? "h3" : "p";
    return r.text.split("\n").filter(l => l.trim()).map(l => `<${tag} class="${esc(r.style)}">${esc(l)}</${tag}>`).join("");
  }
  if (r.kind === "image") {
    const src = (r.data as { src?: string } | null)?.src;
    return `<figure>${src ? `<img src="${esc(src)}" alt="">` : ""}${r.text.trim() ? `<figcaption>${esc(r.text)}</figcaption>` : ""}</figure>`;
  }
  if (!o) return `<p>${esc(r.text)}</p>`;
  let h = "";
  if (r.kind === "table" && r.text.trim()) h += `<p class="cap">${esc(r.text)}</p>`;
  if (o.latex && r.kind !== "table" && r.kind !== "material") {
    h += `<div class="line${o.input ? " input" : ""}">${ml(o.latex)}`;
    if (o.subst_tex && !o.check) h += ` ${ml(`= ${o.subst_tex}`)}`;
    if (o.result_tex && !o.input && !o.check) h += ` ${ml(o.result_tex.startsWith("\\Rightarrow") ? o.result_tex : `= ${o.result_tex}`)}`;
    if (o.check) h += ` <b class="${o.check.pass ? "pass" : "fail"}">${o.check.pass ? "PASS" : "FAIL"}</b>${o.check.ratio != null ? ` <span class="muted">utilisation ${o.check.ratio.toFixed(2)}</span>` : ""}`;
    if (o.note) h += ` <span class="note">${esc(o.note)}</span>`;
    h += `</div>`;
    if (o.check) h += `<div class="muted">${esc(`${o.check.lhs} ${ops[o.check.op] ?? o.check.op} ${o.check.rhs}`)}</div>`;
  }
  const t = o.table;
  if (t) h += `<table><thead><tr>${t.cols.map(c => `<th>${esc(c.name)}${c.unit ? `<br><small>(${esc(c.unit)})</small>` : ""}</th>`).join("")}</tr></thead><tbody>${Array.from({ length: t.rows }, (_, i) => `<tr>${t.cols.map(c => `<td>${esc(c.cells[i] ?? "")}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
  const p = pics.get(r.id);
  if (o.plot && p) h += `<figure><img src="${p.src}" alt="Plot"></figure>`;
  if (o.material) h += `<p class="muted">Material: ${esc(o.material.name)}</p>`;
  if (o.error) h += `<div class="err">⚠ ${esc(o.error.msg)}</div>`;
  return `<section class="r k-${r.kind}">${h}</section>`;
};

/** The sheet as one self-contained web page. */
export const exportHtml = async () => {
  const s = S(), t = s.title, pics = plots();
  const field = (l: string, v: string) => `<td><small>${l}</small><br>${esc(v || "—")}</td>`;
  const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${esc(t.title || s.name)}</title>
<style>body{font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif;color:#1B2733;max-width:860px;margin:2rem auto;padding:0 1rem}table{border-collapse:collapse;margin:.4rem 0}td,th{border:1px solid #C9D1DA;padding:.25rem .6rem;text-align:left;vertical-align:top}th{background:#F3F5F7}.tb td{min-width:8rem}.tb small{color:#5B6878}h2{margin:1.4rem 0 .3rem}h3{margin:1rem 0 .2rem}.note{color:#5B6878;font-style:italic;margin-left:1rem}.ref{color:#5B6878;font-size:.9em}.input{background:#FFF6E0}.line{margin:.35rem 0}.pass{color:#1E7B34}.fail{color:#B42318}.muted{color:#5B6878;font-size:.9em}.err{color:#B42318}figure{margin:.6rem 0}figure img{max-width:100%}math{font-size:1.1em}</style></head><body>
<table class="tb"><tr>${field("Project", t.project)}${field("Calc number", t.number)}${field("Title", t.title || s.name)}${field("Revision", t.rev)}</tr><tr>${field("Prepared by", t.by)}${field("Checked by", t.checked)}${field("Company", t.company)}${field("Date", t.date)}</tr></table>
${s.regions.filter(r => !r.hide).map(r => region(r, pics)).join("\n")}
<p class="muted">Made with Datum Calc.</p></body></html>`;
  const path = await call<string | null>("save_file", { name: fileName("html"), data: html });
  if (path) toast(`Saved ${path}`, true);
};

/** The sheet as a Word document, written by the engine. */
export const exportWord = async () => {
  const pics = [...plots()].map(([id, p]) => [id, p.src, p.w, p.h]);
  const path = await call<string | null>("export_docx", { plots: pics, name: fileName("docx") });
  if (path) toast(`Saved ${path}`, true);
};
