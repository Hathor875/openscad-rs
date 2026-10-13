# GUI Gap Analysis (vs. OpenSCAD)

Assessment of what the egui GUI is still missing compared to OpenSCAD,
prioritized.

## Critical (functional gaps)

1. **No `include`/`use` support in the GUI.**
   `evaluate()` parses a bare string (`Parser::parse(&source)`) with no file
   path context, so `include <lib.scad>` relative to the opened file cannot
   resolve. The path must be passed through to the `Executor` the same way
   the CLI already does it.

2. **No keyboard shortcuts.**
   There is no key handling at all: Ctrl+O / Ctrl+S, **Ctrl+Enter = Run**,
   and F5/F6 (preview vs. full render, as in OpenSCAD) are all missing.

3. **Auto-preview does not react to edits.**
   The `dirty` flag is only set on startup and file open; typing in the
   editor never marks the document dirty, so a manual Run click is required
   after every change. OpenSCAD's F5 re-renders after each edit.

4. **No way to cancel evaluation.**
   The evaluation thread cannot be interrupted; a runaway `while` loop in a
   SCAD file freezes the viewport with no Cancel button.

5. **Errors carry no source locations.**
   Parse errors do not report line numbers, so there is no click-to-jump
   and no error-line highlighting in the editor.

## Important (usability)

6. **Editor lacks line numbers and syntax highlighting.**
   It is a plain `TextEdit::multiline` (monospace only). `egui_code_editor`
   was dropped over an egui version conflict; needs a custom highlighter or
   an egui version pin.

7. **Viewport features missing:**
   perspective/orthographic toggle, view presets (Front/Top/Right/Iso like
   OpenSCAD's toolbar), "Fit to view" / camera reset, grid visibility
   toggle, PNG screenshot export.

8. **Flat menu bar.**
   Bare buttons instead of File/Edit/View menus; no New, Save As,
   recent-files list, or examples from `examples/`.

9. **No status bar.**
   Missing: evaluation time, cursor world coordinates, zoom level, FPS.
   With the ruler in place this is the natural spot for a readout and a
   measure tool (distance between two clicks).

10. **Console is basic.**
    Errors are not clickable (no jump to line), no filters, no copy-all;
    coloring is a naive substring match on "error".

## Nice to have (later)

11. Customizer (parameters in `/* [..] */` tabs), multi-file tabs,
    undo/redo beyond egui defaults, light/dark theme, OBJ/PNG export from
    the menu (the CLI already supports OBJ).

## Suggested order of work

**1 → 3 → 2 → 5 → 7** (features without which the GUI cannot replace the
CLI), then editor line numbers / highlighting.
