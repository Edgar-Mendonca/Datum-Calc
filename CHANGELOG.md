# Changelog

## 0.1.0 — 2026-10-28

The first release of Datum Calc: the window, the calc sheet, saving and opening, printing, and updates from day one.

**The calc sheet**
- A page (A4 or Letter) with a title block: project, calc number, title, prepared by, checked by, date and revision, your company and logo.
- Math, text and check regions, read top to bottom like a calculation on paper. Ctrl+M adds an equation, Ctrl+T a text region; Enter ends an equation and starts the next one.
- Mark regions as inputs (highlighted), headings, notes or references.
- Move regions with Alt+↑ and Alt+↓; undo and redo every change.
- The Math tab types Greek letters, operators and functions into the region you are editing; the Units tab types units.
- This release keeps what you type; the engine that works out the results with their units arrives in 0.1.1.

**Files**
- Sheets save as `.dcalc` files. A damaged file opens with every region that can still be read.
- A recovery copy is kept while you work; recent sheets are listed on the File page.

**Printing**
- File › Print shows the pages: the title block on every page, "Page x of y" at the bottom and the references at the end. Choose Save as PDF to make a PDF.

**The family window**
- Ribbon (Home, Insert, Math, Units, Check, View, Help, and Format for the selected region), Tell me (Ctrl+K), a ? beside every tool, Options, light, dark and auto themes.
- File › Updates checks GitHub for new versions and installs them after checking their SHA-256 checksum.
