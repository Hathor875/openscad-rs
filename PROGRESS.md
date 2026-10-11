# OpenSCAD Rust Rewrite - Progress Summary

## Completed Components

### 1. Project Structure
- Cargo.toml with dependencies (lalrpop, logos, text-size, rowan, etc.)
- Module organization: `ast`, `lexer`, `parser`, `eval`, `geometry`
- Build script for lalrpop grammar processing

### 2. AST Definitions (`src/ast/mod.rs`)
Complete AST node types matching OpenSCAD's Expression.h:
- **Expressions**: Literal, UnaryOp, BinaryOp, TernaryOp, ArrayLookup, Lookup, MemberLookup, FunctionCall, FunctionDef, Let, Assert, Echo, Range, Vector
- **Statements**: ModuleInstantiation, Assignment, ModuleDef, FunctionDef, Include, Use
- **Supporting types**: Assignment, ModuleModifiers, SourceFile, IncludeInfo, UseInfo, Span (using text-size)
- **Operators**: All OpenSCAD operators (LogicalAnd/Or, Arithmetic, Comparison, Bitwise, Shift)

### 3. Lexer (`src/lexer/mod.rs`)
Complete logos-based lexer with:
- All OpenSCAD keywords (module, function, if, else, let, assert, echo, for, each, true, false, undef)
- All operators (<=, >=, ==, !=, &&, ||, <<, >>, +, -, *, /, %, ^, <, >, !, ~, &, |, ?, :, ., ;, {, }, (, ), [, ], ,, =)
- Number parsing (hex, float, int, scientific notation)
- String literals with escape sequences
- Identifiers (ASCII and Unicode)
- Include/use paths (`<path>`)
- Whitespace/comment skipping
- Location tracking with text-size TextSize
- Lifetime parameter for lalrpop compatibility (`Token<'input>`)

### 4. Parser Grammar (`src/parser/grammar.lalrpop`)
LALRPOP grammar with:
- External token type: `crate::lexer::Token<'input>`
- Location type: `text_size::TextSize`
- Error type: `lalrpop_util::ParseError`
- Terminal mappings for all tokens
- Expression precedence hierarchy (13 levels matching OpenSCAD)
- Statement parsing (module instantiation, assignments, definitions)
- Vector/range parsing
- Function calls with arguments

### 5. Parser Integration (`src/parser/mod.rs`)
- Lexer-to-parser token conversion with span tracking
- ParseError type using TextSize locations
- TermParser entry point (for test grammar)

## Current Blockers

### Lifetime Mismatch (lalrpop + logos)
**Root Cause**: Fundamental architectural difference:
- **lalrpop** expects token type with lifetime parameter `'input` for borrowing from input
- **logos** produces owned tokens (regex variants contain matched strings)
- Generated parser code uses `Token<'input>` but logos tokens don't borrow

**Attempted Solutions**:
1. Added lifetime parameter to Token enum (`Token<'input>`)
2. Added PhantomData variant to satisfy "unused lifetime" check
3. Used `enum crate::lexer::Token<'input>` in extern block

**Remaining Issue**: lalrpop 0.22 generated code has lifetime declaration bugs:
- `__Symbol<'input>` declared but `'input` not properly scoped in all trait impls
- `__ToTriple` trait has lifetime issues
- ParserDefinition impl has lifetime scoping problems

## Recommended Next Steps

### Option A: Switch Parser Generator (Recommended)
Use **pest** or **chumsky** which:
- Work naturally with separate lexing/parsing
- Don't require token lifetime parameters
- Have better Rust ergonomics

### Option B: Hand-Written Recursive Descent Parser
- Full control over token handling
- No lifetime mismatches
- Can directly use logos tokens
- More code but simpler mental model

### Option C: Fix lalrpop Upstream
- Contribute fix to lalrpop for external token lifetime handling
- Requires deep understanding of lalrpop codegen
- Long-term solution but delays project

## Project Structure
```
openscad-rust/
├── Cargo.toml
├── build.rs
├── src/
│   ├── lib.rs
│   ├── ast/
│   │   └── mod.rs          # Complete AST definitions
│   ├── lexer/
│   │   └── mod.rs          # Complete logos lexer
│   ├── parser/
│   │   ├── mod.rs          # Parser integration
│   │   └── grammar.lalrpop # LALRPOP grammar (needs fix)
│   ├── eval/               # Empty - for evaluator
│   └── geometry/           # Empty - for geometry kernel
└── PROGRESS.md
```

## Test Status
- Lexer tests: Passing (basic tokens, strings, numbers, operators)
- Parser tests: Blocked by lifetime issues
- AST: Compiles successfully

## Estimated Effort to Complete Parser
- **Option A (pest)**: ~2-3 days to port grammar and integrate
- **Option B (hand-written)**: ~3-5 days for full parser
- **Option C (lalrpop fix)**: Unknown, requires upstream contribution

## Recommendation
**Switch to pest parser generator** for fastest path to working parser. The grammar is already well-defined in lalrpop format and can be translated to pest's PEG syntax. Pest works seamlessly with separate lexers and doesn't have lifetime issues.