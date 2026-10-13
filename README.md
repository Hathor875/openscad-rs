# openscad-rs

[![CI](actions/workflows/ci.yml/badge.svg)](actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

An experimental Rust implementation of the OpenSCAD language and geometry
pipeline. One binary serves both roles: run it with arguments for headless
CLI evaluation and export, or with no arguments to open the desktop GUI
(code editor, live 3D preview, and console).

## Current status

The parser, interpreter, executor, and CSG pipeline are usable for a growing
subset of OpenSCAD models, and the GUI provides an interactive editing and
preview workflow. Development is validated against the official OpenSCAD
example suite and popular libraries such as BOSL2, NopSCADlib, and dotSCAD.

### Implemented

- Pest parser for OpenSCAD expressions, statements, modules, and functions.
- Logos lexer with numbers, strings, comments, operators, and source locations.
- AST and source-file handling for `include` and `use`, including relative
  paths; cyclic `include`/`use` chains are skipped with a warning, matching
  OpenSCAD and the NopSCADlib pattern.
- Variables, functions, modules, positional and named arguments, defaults, and
  lexical environments.
- Control flow: statement `if`, expression `if/else`, ternary expressions,
  `for`, nested `for`, `each`, filtered comprehensions, `let`, `assert`, and
  `echo`.
- Anonymous functions and common OpenSCAD/BOSL2 type and list helpers.
- 2D primitives: `square`, `circle`, `polygon`.
- 3D primitives: `cube`, `sphere`, `cylinder`, `polyhedron`, and approximate
  `text`.
- `linear_extrude` and `rotate_extrude`, including common scale, twist, angle,
  center, and resolution parameters.
- Transformations: `translate`, `rotate`, `scale`, `mirror`, `multmatrix`, and
  `color`.
- Boolean and compound operations: `union`, `difference`, `intersection`,
  `hull`, and `minkowski`.
- STL and OBJ import/export; ASCII and binary STL export.
- OpenSCAD resolution variables `$fn`, `$fa`, and `$fs`.
- OpenSCAD-style evaluation: bad primitive arguments, type mismatches,
  failed assertions, and runaway recursion degrade to `WARNING` messages
  and `undef` instead of aborting the whole model.
- Cooperative cancellation of running evaluations (GUI Cancel button/Esc).
- A regression suite covering parser, interpreter, primitives,
  transformations, CSG, exports, and compatibility features.

### Graphical user interface

Running the binary without arguments opens the GUI (`cargo run`):

- Code editor with line numbers, syntax highlighting, a click-to-jump
  gutter, error-line highlighting, and tab insertion.
- Debounced auto-preview (600 ms after typing) plus manual run
  (F5 / Ctrl+Enter); open/save with Ctrl+O / Ctrl+S; a running evaluation
  can be stopped with the Cancel button or Esc.
- 3D viewport with a Z-up camera matching OpenSCAD, drag to orbit, scroll
  to zoom, an adaptive ground grid with tick labels and colored axes,
  an orthographic/perspective toggle, and Iso/Front/Right/Top/Fit presets.
- Console with evaluation messages and jump-to-line buttons for parse
  errors; status bar showing evaluation time, triangle count, cursor world
  coordinates, zoom, and projection mode.
- STL export directly from the GUI.

### Known limitations

- This is not yet language- or geometry-compatible with all OpenSCAD models;
  see the mass-test results under [Compatibility work](#compatibility-work).
- BOSL2 mostly loads, but individual examples still fail evaluation (for
  example `attachments.scad`), and some official OpenSCAD test files use
  syntax that does not parse yet.
- Manifold CSG results can differ from OpenSCAD in tessellation, topology, and
  occasionally bounds, especially for complex nested booleans.
- GUI gaps: there is no animation timeline, parameter customizer, undo, or
  PNG/screenshot export (eframe 0.24 has no capture API); the camera supports
  orbit and zoom only, and preview uses CPU painter's-algorithm depth sorting
  rather than a depth-buffered GPU pipeline.
- Some advanced builtins, modules, special variables, and import formats are
  still missing.

## Requirements

- Rust stable and Cargo.
- A working C/C++ toolchain and CMake for the `manifold3d` dependency.
- A graphical session (X11, Wayland, or XWayland) for the GUI; the CLI runs
  headless.

## Build and test

From the repository directory:

```sh
cargo test
cargo build --release
cargo run              # GUI (no arguments)
cargo run -- model.scad -o model.stl   # headless CLI
```

If the host does not provide the native dependencies required by `manifold3d`,
run the same commands inside a Linux development container that has a C/C++
toolchain and CMake.

## Continuous integration

Every push to `main` and every pull request runs [the CI
workflow](.github/workflows/ci.yml), which gates on:

| Job | What it checks |
| --- | --- |
| `rustfmt` | `cargo fmt --all -- --check` |
| `clippy` | `cargo clippy --locked --all-targets -- -D warnings` |
| `test` | `cargo test --locked --all-targets` on Linux, macOS and Windows |
| `CLI smoke test` | every `examples/*.scad` evaluates, and the models with geometry export to ASCII STL, binary STL and OBJ with valid headers and sizes |

The same checks run locally:

```sh
cargo fmt --all
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked && ./scripts/ci-smoke.sh
```

The smoke test writes its exports to `out/` and accepts the binary and the
output directory as arguments: `./scripts/ci-smoke.sh <binary> <out-dir>`.

## Installation

Prebuilt binaries are attached to every
[release](https://github.com/Hathor875/openscad-rs/releases). The same binary
provides both interfaces: with arguments it is the headless CLI, without them
it starts the GUI.

```sh
# Linux and macOS
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Hathor875/openscad-rs/releases/latest/download/openscad-rust-installer.sh | sh

# Homebrew
brew install --formula https://github.com/Hathor875/openscad-rs/releases/latest/download/openscad-rust.rb

# npm
npx openscad-rust model.scad -o model.stl
```

Windows binaries are not published yet. Portable `.tar.xz` archives are
attached for every published platform.

Running the GUI on Linux needs the usual desktop libraries present:
`libgtk-3`, `libwayland-client`, `libxkbcommon` and `libGL`. The CLI has no
runtime dependency beyond glibc.

## Cutting a release

Releases are built by [cargo-dist](https://axodotdev.github.io/cargo-dist);
the configuration lives in `dist-workspace.toml` and the workflow in
`.github/workflows/release.yml`.

```sh
# 1. Set the new version in Cargo.toml and record it in CHANGELOG.md
# 2. Commit, then tag and push the tag
git tag v0.2.0
git push origin v0.2.0
```

The tag push builds every target, assembles the archives and installers, and
publishes them to a GitHub Release with checksums.

## Command-line usage

The command accepts one input `.scad` file. With no output option it parses and
evaluates the model; with `-o`/`--output` it exports STL or OBJ.

```sh
cargo run -- model.scad
cargo run -- model.scad -o model.stl
cargo run -- model.scad -o model.stl --binary
cargo run -- model.scad -o model.obj
```

Binary output is supported only for STL. Errors are reported with the input
path and operation that failed, and an empty model is rejected during export.

## Minimal example

```scad
$fn = 32;
difference() {
    cylinder(h = 20, d = 20);
    translate([0, 0, -1])
        cylinder(h = 22, d = 10);
}
```

Save it as `model.scad`, then run:

```sh
cargo run -- model.scad -o model.stl --binary
```

## Project layout

| Path | Responsibility |
| --- | --- |
| `src/lexer` | Logos-based tokenization |
| `src/parser` | Pest grammar and AST construction |
| `src/ast` | Expressions, statements, modules, and source metadata |
| `src/interpreter` | Values, environments, builtins, evaluation, and execution |
| `src/csg` | Meshes, primitives, transforms, booleans, and exporters |
| `src/cancel.rs` | Cooperative cancellation of running evaluations |
| `src/main.rs` | egui GUI, viewport rendering, and CLI entry point |
| `src/interpreter/tests.rs` | End-to-end regression tests |

## Compatibility work

Compatibility is validated two ways.

### Mass compatibility test

`../run_mass_test.py` evaluates every `.scad` file in a ~3700-file corpus
(official OpenSCAD tests plus BOSL2, NopSCADlib, dotSCAD, LEGO.scad,
gridfinity, and others) with the release binary. Current state
(2026-10-13):

| Result | Count | Share |
| --- | --- | --- |
| Evaluates successfully | 3651 | 98.4% |
| — with generated geometry | 1369 | 36.9% |
| Failures (parse/eval) | 37 | 1.0% |
| Timeouts (> 10 s) | 21 | 0.6% |
| Crashes/panics | 0 | 0% |

The full report is written to `../mass_test_report.md` with raw results in
`../mass_test_results.json`.

### Example models

Official OpenSCAD examples such as `Parametric/candleStand.scad` and
`Basics/CSG-modules.scad`, plus selected BOSL2 library models. Matching
triangle counts is not expected: tessellation is implementation-dependent.
Comparisons should use successful evaluation, non-empty output, dimensions,
and visual/topological behavior.

## License

This project is licensed under the GNU General Public License v3.0 or later
(`GPL-3.0-or-later`). See [LICENSE](LICENSE) for the full text.

GPL-3.0 is compatible with the GPL-2.0 license of OpenSCAD and with the
MIT/Apache-2.0 licensed Rust crates used by this project, so the program may be
combined with and merged into the OpenSCAD ecosystem.
