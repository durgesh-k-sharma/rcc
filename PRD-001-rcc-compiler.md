# PRD-001: rcc — A Self-Hosting C Compiler in Rust

## Problem Statement

There is no production-quality C compiler that is both (a) implemented in Rust and (b) built with full understanding and control of every layer of the compilation pipeline. Existing options require either delegating code generation to an external backend (LLVM, Cranelift) or being written in C/C++ (GCC, TinyCC, lcc). Engineers and systems programmers who want to study, extend, or embed a C compiler as a Rust library face a gap: either they accept a dependency they don't fully control, or they build from scratch without a clear architectural roadmap.

## Solution

Build `rcc`, a C17 compiler implemented in Rust that parses C source code, performs semantic analysis and optimization, and emits native x86-64 machine code. The compiler is structured as a five-crate Cargo workspace with clean architectural boundaries (frontend → IR → backend), uses the system assembler and linker for object-file generation, supports incremental feature development, and validates correctness primarily through end-to-end runtime testing.

## User Stories

1. As a developer using rcc, I want to compile a single C file (`int main() { return N; }`) into a working executable, so that I can validate the compiler produces correct machine code end-to-end.
2. As a developer using rcc, I want to compile a multi-file C project with separate translation units, so that I can build real software with modular source organization.
3. As a developer using rcc, I want the compiled output to link with GCC/Clang object files, so that I can incrementally adopt rcc in existing build systems.
4. As a developer using rcc, I want to see source-accurate error messages with file, line, and column information, so that I can quickly locate and fix errors in my C code.
5. As a developer using rcc, I want the compiler to report multiple errors per invocation where possible (with parser recovery), so that I can fix several issues without repeated recompilation.
6. As a developer using rcc, I want correct implementation of C89/C90 core language features, so that I can compile substantial C codebases written to that standard.
7. As a developer using rcc, I want correct implementation of common C99 features (`long long`, designated initializers, inline functions), so that I can compile modern C projects.
8. As a developer using rcc, I want correct implementation of essential C11/C17 features (`_Static_assert`, `_Alignas`, `_Alignof`), so that I can compile code targeting those standards.
9. As a developer using rcc, I want ABI-compliant code generation following the System V AMD64 ABI, so that my compiled code interoperates correctly with system libraries and other compilers.
10. As a developer maintaining rcc, I want a test suite that compiles C snippets, runs the resulting binaries, and asserts on exit codes and output, so that I can validate correctness across the entire pipeline.
11. As a developer maintaining rcc, I want differential testing against GCC/Clang for supported language features, so that I can detect semantic mismatches before they cause user-facing bugs.
12. As a developer maintaining rcc, I want unit tests for compiler internals (type canonicalization, liveness analysis, register allocation), so that I can isolate and debug algorithmic errors.
13. As a developer maintaining rcc, I want a centralized diagnostic infrastructure with source-span tracking from the first version, so that error quality can improve without architectural rewrites.
14. As a developer maintaining rcc, I want snapshot/FileCheck-style tests for IR transformation passes, so that optimizer changes are regression-tested without being as brittle as assembly snapshots.
15. As a developer using rcc, I want basic optimizations (constant folding, dead code elimination, copy propagation) to produce reasonably efficient code, so that compiled programs perform adequately for real use.
16. As a developer using rcc, I want the compiler to use global linear-scan register allocation, so that generated code uses registers efficiently rather than spilling everything to the stack.
17. As a developer using rcc, I want the compiler to exploit x86-64 addressing modes and instruction combining via pattern-matching instruction selection, so that generated code quality approaches that of established compilers.
18. As a developer using rcc, I want support for separate compilation with proper symbol scoping and linkage rules, so that I can build libraries and multi-unit programs.
19. As a developer using rcc, I want correct handling of C's struct/union layout rules including padding and alignment, so that my data structures are compatible with other compilers.
20. As a developer using rcc, I want correct implementation of C's type system including incomplete types, compatible types, and implicit conversion rules, so that subtle semantic issues are handled correctly.
21. As a developer contributing to rcc, I want the project to use a Cargo workspace with clear crate boundaries (frontend, IR, backend, support, driver), so that I can understand and modify the codebase without deep knowledge of every subsystem.

## Implementation Decisions

### Architecture and Crate Structure

The compiler is organized as a Cargo workspace with five crates:

- **rcc-frontend**: source management, diagnostics, lexer, parser, AST, symbol tables, semantic analysis, type system.
- **rcc-ir**: HIR/TAC definitions, control-flow graphs, analysis passes, optimization passes, pass infrastructure.
- **rcc-backend**: machine IR, instruction selection, register allocation, ABI implementation, x86-64 assembly emission.
- **rcc-support**: shared utilities — arena allocation, string interning, source spans, identifiers, diagnostic infrastructure.
- **rcc-driver**: CLI interface, compilation orchestration, toolchain integration (assembler/linker invocation).

### Parsing Strategy

Hand-written recursive descent parser. The lexer always emits `IDENTIFIER` tokens (no lexer hack). When the parser encounters a construct ambiguous between a declaration and an expression (such as `A * B;`), it queries a lightweight symbol table containing typedefs, tags, and scope information to disambiguate. This keeps the lexer simple while remaining compatible with real-world C.

### Source Representation and Diagnostics

Every token, AST node, type reference, and IR construct carries a source span `(file_id, start_offset, end_offset)`. Line and column are computed lazily through a source manager, not stored redundantly. Diagnostics are produced through a centralized subsystem that collects structured reports (severity, primary span, message, secondary spans, optional fix-it hints) from all compiler passes. Initial output targets GCC-quality file:line:col; the infrastructure supports evolution toward Clang-style caret diagnostics without architectural change.

### Type System

Hybrid model: nominal for struct/union/enum types (identified by tag name and scope), structural for everything else (arithmetic types, pointers, arrays, function types). Types are canonicalized as immutable, arena-allocated nodes shared throughout the compiler. Incomplete types are an explicit state (structs without bodies, arrays without size) tracked alongside completion status. Separate namespaces are maintained for identifiers, tags, labels, and typedef names. Semantic analysis resolves all implicit conversions (integer promotion, usual arithmetic conversions, array-to-pointer decay, qualifier propagation) before the IR stage.

### Intermediate Representation

Non-SSA three-address code (TAC) as the primary IR. A target-independent HIR/TAC layer carries semantic information for analysis and optimization. A lower Machine IR handles target-specific concerns. SSA is deliberately deferred — it will be considered as an addition on top of the existing TAC infrastructure once the compiler can reliably compile larger C programs.

### Optimization Pipeline

Staged: basic optimizations (constant folding, copy propagation, dead code elimination, strength reduction) in the first pass. More sophisticated transforms added as profiling identifies opportunities. All optimizations operate on the TAC IR before backend lowering.

### Backend

Staged implementation:
1. **Assembly emission**: direct instruction templates, all temporaries spilled to the stack.
2. **Peephole optimization**: redundant move elimination, instruction simplification.
3. **Linear-scan register allocation**: global allocator integrated with liveness analysis.
4. **Pattern-matching instruction selection**: maximal-munch for addressing-mode folding, compare-and-branch fusion, scaled-index addressing.
5. **Future**: direct ELF object generation via an abstract object-generation interface.

ABI: Full System V AMD64 compliance from day one, including register argument classification (INTEGER, SSE, MEMORY classes). Struct layout follows the ABI with padding, natural alignment, and field ordering. Function types distinguish prototype vs. non-prototype signatures.

### Preprocessor

External preprocessing initially — the compiler invokes the system `cpp` or `clang -E` and consumes the result as a token stream. The token-stream abstraction is designed so that an integrated preprocessor can replace the external pipeline later without affecting other frontend components. Source-location and macro-expansion metadata are preserved in a forward-compatible way.

### Toolchain Integration

The compiler emits textual x86-64 assembly (`.s` files) and invokes the system assembler (`as`) and linker (`ld`). The backend uses an abstract object-generation interface so that direct ELF production can replace assembly emission in the future without changing higher layers.

### Dependencies

Minimal. Core compiler logic (parsing, type system, IR, optimization, backend) has no external dependencies beyond Rust stdlib. Targeted dependencies allowed for: arena allocation, string interning, terminal rendering for diagnostics, and CLI argument parsing. No parser generators, parser-combinator frameworks, or compiler-construction toolkits. No external code generation backends.

### Testing Strategy

Primary: end-to-end runtime tests via a Rust test harness that compiles C programs, links them, executes them, and asserts exit codes/stdout/stderr. Secondary: unit tests for isolated algorithms (type compatibility, liveness, regalloc, data layout), IR snapshot tests for optimization passes, and differential testing against GCC/Clang for semantic validation. A regression suite is maintained for every discovered bug.

## Testing Decisions

### What makes a good test

A good test validates observable behavior of the compilation pipeline, not implementation internals. For runtime tests: compile C source, run the binary, assert on exit code and I/O. For IR tests: assert that a given transformation produces a semantically equivalent IR (snapshot on IR dump, not on assembly output). Implementation details (which registers are used, how spills are scheduled, instruction ordering within a block) are intentionally not tested directly — they are covered by the correctness of the final binary output.

### Modules under test

- **E2E tests (rcc-driver)**: full compilation of C programs, execution verification.
- **Unit tests (rcc-frontend)**: type canonicalization, scope resolution, struct layout calculations, diagnostic infrastructure.
- **Unit tests (rcc-ir)**: liveness analysis, constant folding, dead code elimination, control-flow graph construction.
- **Unit tests (rcc-backend)**: register allocation decisions (correctness, not quality), ABI argument classification, data-layout computations.
- **Snapshot tests (rcc-ir)**: IR dumps before and after optimization passes.
- **Differential tests (project-level)**: same source compiled with rcc vs. GCC/Clang, behavior comparison.

### Prior art

- Rust's `compiletest` framework for compiler testing (though rcc is simpler and does not need the full harness).
- LLVM's FileCheck for IR-level assertions.
- TCC's test suite as a model for concise, executable C conformance tests.

## Out of Scope

- SSA-based optimization passes (deferred until the compiler compiles larger programs reliably).
- Direct ELF object-file generation (deferred until assembly emission is stable).
- Integrated preprocessor (deferred until the core pipeline is functional; external preprocessor used initially).
- Custom linker (out of scope entirely — system `ld` is always used).
- Graph-coloring register allocation (may be revisited if linear scan is shown to be a bottleneck).
- GNU C extensions beyond those required to compile real-world target software (added only on demand, based on compatibility requirements).
- `_Atomic` types and thread-local storage (C11 features deferred).
- Variable-length arrays (deferred until core type system and ABI are proven).
- Cross-compilation to non-x86-64 targets (not in scope for the first release).
- LSP server integration (downstream of the diagnostic infrastructure but not planned for initial implementation).

## Further Notes

The project started from an empty directory — there is no existing code, no parse-trees, and no architectural legacy to work around. The first file to write is the source manager and diagnostics infrastructure, not the parser. The first executable milestone is compiling `int main() { return 42; }` into a binary that exits with status 42. From there, the progression is: function calls, multi-file projects, structs, pointers, control flow, then increasingly realistic C software.

The compiler is written in Rust and will not self-host in the traditional sense (the compiler itself stays in Rust). Success is measured by what C software it can compile, not by compiling itself.
