# openscad-rs

A Rust implementation of OpenSCAD, an open-source CAD program.

## Status

This is a work-in-progress rewrite focusing on the interpreter, parser, and CSG operations.

### Implemented
- **Lexer**: Tokenization using logos
- **Parser**: Pest-based grammar for OpenSCAD syntax
- **AST**: Core AST types for expressions, statements, modules, functions
- **Interpreter**: Evaluation engine with environment, functions, modules, builtins (math, etc.)
- **Executor**: Statement execution with CSG stack for transformations and boolean ops
- **CSG**: Mesh representation with proper boolean operations (union/difference/intersection) using manifold3d
- **Builtins**: Basic mathematical functions, trigonometry, etc.
- **Primitives**: cube, sphere, cylinder, square, circle, polyhedron support
- **Transformations**: translate, rotate, scale, mirror
- **Boolean operations**: union, difference, intersection, hull, minkowski
- **Control flow**: if statements, for/each expressions (partial), let, assert, echo

### Dependencies
- pest/pest_derive - Parser generator
- logos - Lexer
- nalgebra - Linear algebra
- manifold3d - High-quality CSG operations
- text-size, rowan - Source location handling
