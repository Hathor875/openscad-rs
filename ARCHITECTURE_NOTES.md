# OpenSCAD Rust - Architecture & Status Notes

## 1. Current Architecture
* **Parser**: Successfully migrated to `pest` (`openscad.pest` and `src/parser/mod.rs`). The old `lalrpop` setup is completely bypassed.
* **Interpreter**: Cleanly separated into expression evaluation (`eval.rs`), program execution (`exec.rs`), and built-in functions (`builtins.rs`).
* **CSG Engine**: A foundational Mesh-based engine exists (`src/csg/mod.rs`) with basic exports to OBJ/STL.

## 2. Technical Debt (Relics to Remove)
You can safely clean up the project by deleting the old parser/lexer files:
* Delete `src/parser/grammar.rs` (old auto-generated lalrpop file).
* Delete the `src/lexer` directory entirely (old logos implementation).
* Remove `pub mod lexer;` from `src/lib.rs`.
* Remove `lalrpop` and `logos` dependencies from your `Cargo.toml`.

## 3. Immediate Next Steps (The CSG Blocker)
* **Real Boolean Operations**: Currently, the `union` function just blindly concatenates vertices, and `difference` / `intersection` are just placeholders returning cloned meshes. You will need to implement a real CSG math engine (e.g., using BSP trees) or pull in a crate like `parry3d` to calculate actual mesh intersections.
* **Mesh Generation**: Cylinder and sphere generations are simplified and need proper pole/cap handling.
