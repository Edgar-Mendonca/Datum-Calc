#!/usr/bin/env python3
"""Builds crates/core/data/materials.json from a Datum Atlas checkout (MIT, github.com/Edgar-Mendonca/Datum-Atlas).

    python3 tools/materials.py path/to/datum-atlas

Keeps the engineering properties Calc can use (see KEYS in crates/core/src/materials.rs) with their
ranges and sources, the materials' names, families, synonyms and designations, and the Atlas commit."""
import json, pathlib, subprocess, sys

KEEP = ["rho", "E", "G", "nu", "sy", "ts", "sc", "elong", "hv", "fat", "tm", "tg", "tmax", "tmin", "lam", "alpha", "cp", "res", "eps"]
atlas = pathlib.Path(sys.argv[1])
commit = subprocess.run(["git", "-C", str(atlas), "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
sources = {s["id"]: s for s in json.loads((atlas / "data/sources.json").read_text())["sources"]}
mats, used = [], set()
for f in sorted((atlas / "data").glob("*.json")):
    if f.name in ("sources.json", "processes.json"):
        continue
    for m in json.loads(f.read_text()).get("materials", []):
        props = {k: [v[0], v[1], v[2]] for k, v in m["props"].items() if k in KEEP and len(v) == 3 and all(isinstance(x, (int, float)) for x in v[:2])}
        if not props:
            continue
        used.update(v[2] for v in props.values())
        # "EN AW-6061" already names its standard: not "EN EN AW-6061".
        des = ", ".join(v if str(v).startswith(k) else f"{k} {v}" for k, v in m.get("des", {}).items())
        mats.append({"id": m["id"], "name": m["name"], "family": m["family"][0], "syn": m.get("syn", []), "des": des, "status": m.get("status", ""), "notes": m.get("notes", ""), "props": props})
out = {
    "from": f"Datum Atlas {commit} (MIT); each value names its source",
    "sources": {k: {"title": s["title"], "kind": s.get("kind", ""), "note": s.get("note", "")} for k, s in sources.items() if k in used},
    "materials": mats,
}
path = pathlib.Path(__file__).resolve().parent.parent / "crates/core/data/materials.json"
path.parent.mkdir(exist_ok=True)
path.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")) + "\n")
print(f"{len(mats)} materials, {len(out['sources'])} sources, {path.stat().st_size // 1024} KB → {path}")
