# Datum Calc: notes for Claude

Read `docs/MEMORY.md` first. It is the project's memory: architecture, decisions, roadmap, and a log of every session.
At the end of every session that changes anything, update `docs/MEMORY.md`:
- tick the roadmap
- add a session-log entry
- record any new decisions
- refresh the architecture section if files moved

The family design guide is `FAMILY.md` in the hub (`Edgar-Mendonca/Datum`); `docs/FAMILY.md` links to it. Datum Plan (`Edgar-Mendonca/Datum-Plan`) is the reference implementation: copy from it rather than reinventing.

User preferences:
- Rust and Tauri 2, small and efficient code, Windows first.
- Ship in small numbered releases (0.1.0, 0.1.1, …). Every release appears in the in-app updater.
- Amber accent (`#A8650F`); warnings use the warning colours with an icon, never the accent alone.
- Keep code compact and match the surrounding style. Engine logic belongs in `crates/core` (pure Rust, unit-tested), not in the UI.
- The UI is SolidJS + Vite in `ui/src/*.tsx` (built to `ui/dist`). State lives in `ui/src/store.ts`; the engine computes all values, the window only shows them.
- Public words (FAMILY §13): describe what Calc does; never name, compare with, or say it copies or replaces a commercial product, in any file, commit, release note or the site.

Checks before pushing:
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run check:ui`, then `npm run build:ui` and the UI tests (`cargo run -q -p datum-core --example dump > tests_ui/fixture.json && node tests_ui/run.cjs`)
- `cargo deny check` (licences, advisories, sources; CI runs it)

Releasing:
1. Bump the version in `src-tauri/Cargo.toml` and `crates/core/Cargo.toml`.
2. Add a CHANGELOG section (`## x.y.z — date`), and keep `docs/site.json` true to the release (`about`, `steps`, `highlights`, `roadmap`; no commercial product names).
3. Push, then run `release.yml` with workflow_dispatch (or push the tag `vX.Y.Z`). The workflow creates the tag and publishes `DatumCalc-x.y.z-setup.exe` and `SHA256SUMS.txt`.
