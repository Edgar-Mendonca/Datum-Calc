# Security

## Reporting a problem

Please report a security problem privately: use **Security › Report a vulnerability** on this repository (GitHub private advisory), or open an issue that only says you have something to report and we will contact you. Do not post details in a public issue.

## What Datum Calc does and does not do

- Calc sheets are `.dcalc` files (JSON) on your own computer. Keep them where you keep your other engineering documents.
- The only network call is the update check and download from this repository's GitHub Releases. Every download is checked against its SHA-256 checksum before it installs.
- The window can only open links to this repository and the Datum family site (`open_url` refuses anything else).
- Opened files are treated as untrusted: the reader is tested with thousands of damaged files and must never crash the app; a damaged sheet opens with every region that can still be read.
- Equations are evaluated by Datum Calc's own engine; nothing typed in a sheet is ever run as a program. Lines are limited in length and nesting, and the engine is tested with over 100,000 random sheets, so no line can stop the app.
- The installer is not code-signed yet (planned for 1.0), so Windows shows a SmartScreen warning on the first install.
