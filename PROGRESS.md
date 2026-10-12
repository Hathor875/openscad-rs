# Development status

## Current milestone

The project has a working headless path:

```text
.scad source -> lexer/parser -> AST -> interpreter/executor -> CSG mesh -> STL/OBJ
```

The regression suite currently contains 69 tests. The latest compatibility
work adds BOSL2-oriented type/list builtins (`is_def`, `is_vector`, `is_int`,
`is_finite`, `is_nan`, and `last`) and keeps function/module captures shared
through reference-counted environments to avoid copying large libraries.

## Next compatibility target

The next language task is parsing a BOSL2 comprehension such as:

```scad
[for (...) if (condition)
    if (condition_a) value_a
    else if (condition_b) value_b
]
```

This syntax currently blocks complete loading of `BOSL2/std.scad` through
`attachments.scad`. Once it is supported, the threaded `screw("M8")` and
`nut("M8")` model can be evaluated end to end and compared with OpenSCAD.

After language compatibility, the main geometry task is improving CSG parity:
boolean robustness, manifold validity, tessellation, and preservation of
holes and bounds.

## Validation commands

```sh
cargo test
cargo run -- model.scad -o model.stl --binary
cargo run -- model.scad -o model.obj
```

When native dependencies are not available on the host, use the
`distrobox enter openscad-rust` command documented in `README.md`.
