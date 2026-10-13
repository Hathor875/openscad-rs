# openscad-rs

An experimental Rust implementation of the OpenSCAD language and geometry
pipeline. The project is currently a **headless interpreter and mesh exporter**,
not a replacement for the OpenSCAD GUI.

## Current status

The parser, interpreter, executor, and CSG pipeline are usable for a growing
subset of OpenSCAD models. The implementation is developed against official
OpenSCAD examples and the BOSL2 library.

### Implemented

- Pest parser for OpenSCAD expressions, statements, modules, and functions.
- Logos lexer with numbers, strings, comments, operators, and source locations.
- AST and source-file handling for `include` and `use`, including relative paths
  and include-cycle detection.
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
- A regression suite covering parser, interpreter, primitives, transformations,
  CSG, exports, and compatibility features.

### Known limitations

- This is not yet language- or geometry-compatible with all OpenSCAD models.
- BOSL2 loading has progressed through much of `std.scad`, but a filtered
  comprehension containing a multi-level `if/else if` expression is still a
  parser blocker in `attachments.scad`.
- Manifold CSG results can differ from OpenSCAD in tessellation, topology, and
  occasionally bounds, especially for complex nested booleans.
- The renderer is headless; there is no GUI, preview, editor, animation, or
  interactive parameter control.
- Some advanced builtins, modules, special variables, and import formats are
  still missing.

## Requirements

- Rust stable and Cargo.
- A working C/C++ toolchain and CMake for the `manifold3d` dependency.

The project is also tested in the development container used by this
repository:

```sh
distrobox enter openscad-rust -- bash -lc \
  'export PATH="/usr/local/cargo/bin:$PATH"; \
   cd /home/krzysztofc/Project/openscad/openscad-rust; \
   cargo test'
```

## Build and test

From the repository directory:

```sh
cargo test
cargo build --release
```

If the host does not provide the native dependencies required by `manifold3d`,
run the same commands through the development container shown above.

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
| `src/main.rs` | Headless CLI |
| `src/interpreter/tests.rs` | End-to-end regression tests |

## Compatibility work

Compatibility is validated with official OpenSCAD examples such as
`Parametric/candleStand.scad` and `Basics/CSG-modules.scad`, plus selected BOSL2
library models. Matching triangle counts is not expected: tessellation is
implementation-dependent. Comparisons should use successful evaluation,
non-empty output, dimensions, and visual/topological behavior.

## License

This project is licensed under the GNU General Public License v3.0 or later
(`GPL-3.0-or-later`). See [LICENSE](LICENSE) for the full text.

GPL-3.0 is compatible with the GPL-2.0 license of OpenSCAD and with the
MIT/Apache-2.0 licensed Rust crates used by this project, so the program may be
combined with and merged into the OpenSCAD ecosystem.
