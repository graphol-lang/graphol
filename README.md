# Graphol — Towards a New Language

Graphol is a Graph Oriented Language, with the compiler/interpreter written in Rust.

## Build and Run (Interpreter vs Compiled Binary)

Use this section as a quick reference for day-to-day usage.

### 1. Build the Graphol tool itself

This builds `graphol` (the CLI that can interpret `.graphol` files and also generate executables):

```bash
make build
```

Install the binary after build:

```bash
sudo make install
```

### 2. Run a `.graphol` program in interpreter mode (no `-o`)

In this mode, `graphol` reads and executes the source file directly:

```bash
graphol examples/program5.graphol
```

You can also pipe source through stdin:

```bash
cat examples/program5.graphol | graphol
```

### 3. Compile a `.graphol` program to a Linux executable (`-o` / `--output`)

In this mode, `graphol` generates a standalone Linux executable for the provided Graphol source:

```bash
graphol examples/program5.graphol -o program5
```

Equivalent long flag:

```bash
graphol examples/program5.graphol --output program5
```

Run the generated executable:

```bash
./program5
```

### 4. Use a directory as input (defaults to `main.graphol`)

If the input path is a directory, `graphol` automatically uses `<directory>/main.graphol` as the entry file.

Run in interpreter mode:

```bash
graphol examples
```

Compile from directory entry:

```bash
graphol examples -o program_from_examples
```

### Difference between using `-o/--output` or not

- Without `-o/--output`: runs as an interpreter immediately and does not create a new executable file.
- With `-o/--output`: compiles the `.graphol` source into a native Linux executable at the specified output path, which you run directly later.

# Graphol Rust Codebase Summary

## Scope of this analysis

This summary was produced after reading:

- The entire current repository tree (Rust source, tests, examples, manifest files, and ignore rules)
- The last `README.md` (which describes a legacy JavaScript architecture)
- Repository history around the migration (`HEAD` and 3-5 commits behind)

## Historical context: why the current README is outdated

- `6704b16` added `README.md` with a full explanation of the old JavaScript browser prototype (`PrototipoJS/...`).
- `4970ec4` (current `HEAD`) performed a one-shot migration to Rust (`src/...`, `tests/...`, `examples/...`) and removed the JS runtime/compiler files.
- 3-5 commits before `HEAD` (for example: `cd1309e`, `4cf67af`, `bf417fc`, `ec9fd95`) still belonged to the JS era, and the legacy `README` file was effectively empty.

Conclusion: `README.md` was generated for the previous codebase and was not updated hitherto.

## What the project is now

Graphol is now a Rust implementation of a small message-passing language runtime. It includes:

- A parser that builds an AST directly from Graphol source
- An in-memory VM with scope chaining and cooperative multi-thread scheduling
- Dynamic object strategies (node behavior determined by first received value)
- Built-in commands/messages (`input`, `echo`, `stdout`, `if`, `run`, `async`, `else`)
- Example `.graphol` programs and automated runtime tests

## End-to-end execution flow

1. Source is read from a file argument or `stdin` (`src/main.rs`).
2. `parse_program` parses text into `Program`/`Expr`/`NodeExpr` AST structures (`src/parser.rs`, `src/ast.rs`).
3. `Vm::new` is created with parsed program plus IO backend (`src/runtime/vm.rs`, `src/runtime/io.rs`).
4. VM initializes a root scope with built-ins (`src/runtime/scope.rs`).
5. Each expression is evaluated as:
   - First node = receiver object
   - Remaining nodes = messages sent to receiver
6. Objects mutate behavior via strategy objects (`src/runtime/object/...`).
7. `echo` emits outputs through `RuntimeIo`; `input` reads from IO backend.
8. Blocks can execute synchronously (`run`) or spawn async threads (`async run`).

## Current repository structure

- `src/`: language front-end + runtime implementation
- `tests/`: runtime-level integration tests
- `examples/`: sample Graphol programs
- `Cargo.toml` / `Cargo.lock`: crate metadata and lockfile
- `.gitignore`: NetBeans leftovers + `target/`
- `README.md`: Graphol Rust Codebase Summary

## File-by-file analysis (current tree)

### Root files

- `Cargo.toml`: crate `graphol`, edition 2024, no external dependencies.
- `Cargo.lock`: lockfile with only the local package.
- `.gitignore`: ignores `nbproject` folders and `target`.
- `README.md`: legacy JS documentation; does not represent current Rust architecture.

### Entry points and API

- `src/main.rs`:
  - CLI entry point.
  - Reads source from `argv[1]` or `stdin`.
  - Parses and runs VM with `StdIo`.
- `src/lib.rs`:
  - Public modules: `ast`, `parser`, `runtime`.
  - Exposes `run_graphol(source, io)` convenience API.
  - Wraps parse/VM errors into `GrapholError`.

### AST and parser

- `src/ast.rs`:
  - Defines `Program`, `Expr`, `NodeExpr`, `BlockLiteral`.
  - Defines reserved operator enums:
    - `ArithmeticOp` (`+ - * / ^`)
    - `LogicOp` (`= != > < >= <=`)
    - `BooleanOp` (`& | ! x|`)
- `src/parser.rs`:
  - Hand-written character parser (no parser generator).
  - Produces AST directly.
  - Supports:
    - Identifiers
    - String literals with escape handling
    - Parenthesized sub-expressions
    - Block literals `{ ... }`
    - Reserved operators/tokens
  - Emits `ParseError { message, position }`.

### Runtime module surface

- `src/runtime/mod.rs`: exports runtime submodules and re-exports VM/IO public types.
- `src/runtime/host.rs`: `ExecutionHost` trait used by objects to request side effects.
- `src/runtime/value.rs`:
  - Runtime `Value` union (`Obj`, `Number`, `Text`, `Bool`, `Null`).
  - Conversion helpers (`as_text`, `as_number`, `as_bool`, `to_scalar`).
- `src/runtime/io.rs`:
  - `RuntimeIo` trait abstraction.
  - `StdIo` for interactive CLI.
  - `TestIo` for deterministic tests.
  - `OutputMode` (`Alert`, `Console`) and `OutputEvent`.
- `src/runtime/scope.rs`:
  - Hierarchical scope with parent lookup.
  - Initializes built-ins in every new scope.
  - Supports `find`, `get` (auto-create node), and `set`.

### Object system and strategies

- `src/runtime/object.rs`:
  - Core `GrapholObject` trait.
  - `MessageKind` enum (`Run`, `Async`, `Else`).
  - `BlockSnapshot` transport object for block invocation.
  - `StdoutState` shared mutable output-mode state.
  - Utility wrappers for send/exec/end/message-kind extraction.
- `src/runtime/object/object_commands.rs`:
  - Block object (`run`, `async`, `inbox` behavior).
  - Commands: `input`, `echo`, `stdout`, `if`.
  - Message objects: `run`, `async`, `else`.
  - `if` command state machine supports chained conditions and optional `else`.
- `src/runtime/object/object_strategies.rs`:
  - Strategy module aggregator/re-export.
- `src/runtime/object/object_strategies/strategy_core.rs`:
  - Splits primitive vs numeric strategy modules.
- `src/runtime/object/object_strategies/node_primitives.rs`:
  - Generic node with late-bound strategy (`NodeObject`).
  - String and boolean strategies.
  - Strategy factory that maps first received value/type to concrete strategy.
- `src/runtime/object/object_strategies/numeric_ops.rs`:
  - Number strategy with operator handoff.
  - Arithmetic operator strategy implementing accumulation.
  - Supports XOR via integer cast.
- `src/runtime/object/object_strategies/strategy_predicates.rs`:
  - Logic comparator strategy (type-aware comparisons).
  - Boolean operator strategy (`and`, `or`, `not`, `xor`).

### Virtual machine

- `src/runtime/vm.rs`:
  - Main scheduler/executor.
  - Maintains multiple threads, each with frame stacks.
  - Evaluates expressions using receiver/message semantics.
  - Converts AST nodes into runtime values/objects.
  - Creates child scopes for blocks and injects `inbox`.
  - Executes sync blocks inline on current thread.
  - Enqueues async blocks as separate threads.
  - Captures emitted output events and forwards to IO backend.

### Tests and examples

- `tests/graphol_runtime.rs`:
  - Integration tests for arithmetic, blocks/inbox, conditionals/else, async execution.
  - Uses `run_graphol` + `TestIo`.
- `examples/program.graphol`: basic variables, input, nested echo, and expression usage.
- `examples/program2.graphol`: arithmetic demos.
- `examples/program3.graphol`: block + `inbox` + `input`.
- `examples/program4.graphol`: conditionals and boolean logic.
- `examples/program5.graphol`: async block execution and output mode switch.
- `examples/program6.graphol`: conditional/boolean scenario duplicated from program4-style content.

## How files communicate with each other

- `main.rs` depends on `parser` + `runtime::Vm`.
- `lib.rs` orchestrates `parse_program` -> `Vm::run`.
- `parser.rs` produces AST types from `ast.rs`.
- `vm.rs` is the central consumer of AST and producer of runtime behavior.
- `vm.rs` relies on:
  - `scope.rs` for symbol lookup and built-ins
  - `object.rs` and strategy/command submodules for dynamic behavior
  - `value.rs` for value transport/conversions
  - `io.rs` for side effects
- `object_commands.rs` and strategy modules call back into VM behavior only through `ExecutionHost` (`host.rs`), keeping object logic decoupled from VM internals.
- Tests validate language behavior through the public API (`run_graphol`) rather than internal modules.

## Functional coverage of the Rust implementation

- Dynamic node typing based on first message
- Numeric, string, boolean, logic, and arithmetic operator semantics
- Blocks with lexical parent scope + `inbox`
- `if`/`else` control flow
- Sync and async block execution model
- Pluggable IO with event capture

## Overall assessment

The project is a focused Rust runtime rewrite of Graphol’s original prototype semantics.
Core language behavior (message passing, node strategies, blocks, conditionals, async-like scheduling, IO commands) is implemented and tested.
