<div align="center">

<img src="docs/brand/datum-calc-icon.svg" width="88" alt="Datum Calc">

# Datum Calc

**Engineering calculations with real units, as readable maths, with checks and a printable calc sheet.**

[![Latest release](https://img.shields.io/github/v/release/Edgar-Mendonca/Datum-Calc?label=release&color=A8650F)](https://github.com/Edgar-Mendonca/Datum-Calc/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/Edgar-Mendonca/Datum-Calc/ci.yml?label=CI&logo=githubactions&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Calc/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/github/downloads/Edgar-Mendonca/Datum-Calc/total?color=A8650F)](https://github.com/Edgar-Mendonca/Datum-Calc/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-A8650F)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D4?logo=windows&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Calc/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-engine-B7410E?logo=rust&logoColor=white)](crates/core)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![SolidJS](https://img.shields.io/badge/SolidJS-UI-2C4F7C?logo=solid&logoColor=white)](ui/src)

[![Download for Windows](https://img.shields.io/badge/Download_for_Windows-A8650F?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Calc/releases/latest)
[![What's new](https://img.shields.io/badge/What%27s_new-D98E1F?style=for-the-badge&logo=readthedocs&logoColor=white)](CHANGELOG.md)
[![Report an issue](https://img.shields.io/badge/Report_an_issue-C0392B?style=for-the-badge&logo=github&logoColor=white)](https://github.com/Edgar-Mendonca/Datum-Calc/issues/new)
[![Memory and roadmap](https://img.shields.io/badge/Memory_%26_roadmap-0C1A2B?style=for-the-badge&logo=bookstack&logoColor=white)](docs/MEMORY.md)

</div>

<img src="docs/images/pack.png" alt="Datum Calc: a pump curve against the system curve, with the operating point solved and checked" width="100%">

## What it does

Datum Calc is a desktop app for engineering calculation sheets. You write the calculation top to bottom, as on paper: an equation per line with its units, text for headings and references, and checks against your limits. It prints as a clean calc sheet with a title block and page numbers, ready to hand in.

1. **Write the equations.** `Q = 120 m^3/h`, `H = 45 m`, `P = rho*g*Q*H/eta`. Greek letters and subscripts are typed simply (`sigma_allow`).
2. **Check the results.** Every answer carries its unit, and `check sigma <= sigma_allow` shows PASS or FAIL with the utilisation ratio.
3. **Print the calc sheet.** Fill in the title block (project, calc number, by, checked, date, revision) and save a PDF.

**Ten calc packs** start you off (File › New from pack), each with its inputs at the top, its results and checks below and its references at the end, and each checked against a hand calculation in the tests:

| Rotating equipment | Process | Mechanical | Electrical |
| --- | --- | --- | --- |
| Pump power, efficiency and motor size · NPSH available against required · Pump curve against system curve (operating point) · Compressor polytropic head and power | Pipe pressure drop (Darcy–Weisbach, Colebrook) · Heat exchanger duty and LMTD | Thin-walled pressure vessel · Beams: deflection and bending stress · Bolt preload and tightening torque | Electric motor: current, power factor and starting current |

<table><tr>
<td width="50%"><img src="docs/images/print.png" alt="The printed calc sheet with its title block"><br><sub>The printed calc sheet: title block, maths, results, PASS/FAIL checks, page numbers.</sub></td>
<td width="50%"><img src="docs/images/units.png" alt="Equations with units and the Variables panel"><br><sub>Inputs with units and descriptions; every name, value and unit in the Variables panel.</sub></td>
</tr></table>

It is for mechanical, process, structural and electrical engineers, students and lecturers: everyone who writes calculations. It is part of the [Datum family](https://edgar-mendonca.github.io/Datum/) of free, open-source desktop tools for engineers and scientists; its page on the family site is [edgar-mendonca.github.io/Datum/apps/calc](https://edgar-mendonca.github.io/Datum/apps/calc).

## Where it is going

| Version | What |
| --- | --- |
| 0.1.0 – 0.1.3 ✅ | The calc sheet and its printing; units, equations, checks, solve and plot; tables and pictures; ten calc packs |
| 0.1.4 | Fixes from trying every pack on a clean Windows PC; a Getting Started sheet |
| 0.2 | Materials from Datum Atlas, more packs, equations shown with their numbers substituted |
| 0.3 – 1.0 | Matrices and ODEs, templates, a validation report, a signed installer |

File › Roadmap in the app shows the full list; it comes from [`docs/site.json`](docs/site.json).

## Install

Download `DatumCalc-x.y.z-setup.exe` from the [latest release](https://github.com/Edgar-Mendonca/Datum-Calc/releases/latest) and run it. It installs for your user only, in seconds, and updates itself from File › Updates. Each release has a `SHA256SUMS.txt` to check the download. Datum Home (from the family site) can install and update it too.

Free and open source (MIT). Windows first. No accounts, no telemetry: your files stay on your computer, and the only network call is the update check.

## Build from source

```bash
npm ci
npm run build:ui
cargo test --workspace
npx tauri build --bundles nsis
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the checks, and [docs/MEMORY.md](docs/MEMORY.md) for the architecture and decisions.
