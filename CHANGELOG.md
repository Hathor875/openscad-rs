# Changelog

## [0.1.0]

First public release. The project is an experimental reimplementation of the
OpenSCAD language in Rust and is not yet a drop-in replacement for OpenSCAD.

### Interpreter and geometry

- Pest parser for OpenSCAD expressions, statements, modules and functions.
- Variables, functions, modules, argument passing, and lexical environments.
- Control flow: `if`, `for`, `each`, `let`, `assert`, `echo`, comprehensions,
  anonymous functions.
- 2D primitives: `square`, `circle`, `polygon`.
- 3D primitives: `cube`, `sphere`, `cylinder`, `polyhedron`, `text`.
- `linear_extrude` and `rotate_extrude` with the common parameters.
- Transformations: `translate`, `rotate`, `scale`, `mirror`, `multmatrix`,
  `color`.
- Booleans and compounds: `union`, `difference`, `intersection`, `hull`,
  `minkowski`.
- STL and OBJ import and export, ASCII and binary STL.
- Resolution variables `$fn`, `$fa` and `$fs`.

### Interfaces

- Headless CLI: `openscad-rust model.scad -o model.stl`.
- egui/wgpu GUI with viewport, source editor and auto-preview.

### Limitations

- OpenSCAD is not yet fully compatible; some builtins, modules and import
  formats are missing.
- The GUI on Linux requires GTK, Wayland/X11 and OpenGL system libraries.