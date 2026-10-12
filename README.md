# miniargs

A small, zero-dependency argument parser for Rust.

## Why

`clap` is quite heavy and its derive API depends on `syn`, which adds seconds to a clean build. `miniargs` has zero dependencies and no proc macros.

## Design goals

1. **Fast compile, zero or minimal deps.** No proc macros, no `syn`.
2. **Small.** One file.
3. **Convenient.** Typed keys, DRY declarations, auto help.
4. **Correct on non-UTF-8 input.** `PathBuf` and `OsString` accept arbitrary bytes, while `String` requires UTF-8.

## Installation

```
cargo add miniargs
```

## Example

```rust
use std::path::PathBuf;

use miniargs::{parse_args, Arg, ArgParser};

#[allow(dead_code)]
#[derive(Debug)]
struct Args {
    target: String,
    jobs: u32,
    project_name: Option<String>,
    log: Option<PathBuf>,
    verbose: bool,
    include: Vec<PathBuf>,
    output: PathBuf,
    inputs: Vec<PathBuf>,
}

fn main() {
    let mut parser = ArgParser::new("pack");
    parser.set_version(env!("CARGO_PKG_VERSION"));

    let args = parse_args!(parser; Args {
        target: required(Arg::new().short('t').desc("Build target")),
        jobs: default(1, Arg::new().short('j').desc("Parallel jobs").type_name("count")),
        // `project_name` becomes `--project-name` on the command line (all '_' become '-').
        project_name: optional(Arg::new().desc("Project name (defaults to directory name)")),
        log: optional(Arg::new().desc("Log file path")),
        verbose: flag(Arg::new().short('v').desc("Verbose output")),
        include: repeated(Arg::new().short('I').desc("Add include directory")),
        output: positional(Arg::new().desc("Output file")),
        inputs: positional_variadic(Arg::new().desc("Input files")),
    });

    if args.inputs.is_empty() {
        parser.die("at least one input file is required");
    }
    if args.include.len() > 3 {
        parser.die(format!("at most 3 --include, got {}", args.include.len()));
    }

    println!("{args:#?}");
}
```

Run with `--help` to see the generated usage text.

## How `parse_args!` works

```rust
let args = parse_args!(parser; Args {
    file_name: required(Arg::new().short('f').desc("File to process")),
});
```

expands to:

```rust
let args = {
    let file_name = parser.required(
        kebab("file_name"),
        Arg::new().short('f').desc("File to process"),
    );
    let args = parser.parse(std::env::args_os());
    Args { file_name: file_name.get(&args) }
};
```

`kebab("file_name")` returns `"file-name"`, so the argument is `--file-name` on the command line while the struct field stays `file_name` in Rust. Names without underscores pass through unchanged. Raw identifiers (`r#move`) lose the `r#` prefix, so `r#move` becomes `--move`.

The struct field type drives the parser type (`file_name: String` instantiates `required::<String>`), so nothing is written twice and there are no turbofish annotations.

The parser is passed explicitly because `macro_rules!` hygiene prevents the macro body from referring to a caller binding by name.

For tests, use the two-argument form with an explicit argv:

```rust
let argv = vec![OsString::from("pack"), OsString::from("-v"), OsString::from("out.bin")];
let args = parse_args!(parser, argv; Args {
    verbose: flag(Arg::new().short('v')),
    output: positional(Arg::new()),
});
```

`std::env::args_os()` is process-global and unsafe to mutate in parallel tests, so the explicit-argv form is the one you want in `#[test]` functions.

## Argument shapes, by example

**Required named option.** Missing it is an error.

```rust
target: required(Arg::new().short('t'))
// --target x
// -t x
```

**Defaulted named option.** Absent means the default.

```rust
jobs: default(1u32, Arg::new().short('j'))
// omitted      → 1
// --jobs 8     → 8
```

**Optional named option.** Absent means `None`.

```rust
log: optional(Arg::new().short('l'))
// omitted   → None
// --log x   → Some(PathBuf::from("x"))
```

**Boolean flag.** Present means `true`.

```rust
verbose: flag(Arg::new().short('v'))
// omitted    → false
// -v         → true
// --verbose  → true
```

**Repeated option.** Every occurrence accumulates.

```rust
include: repeated(Arg::new().short('I'))
// -Ia -I b --include=c → [a, b, c]
// omitted              → []
```

**Aliases.** Alternate long names, accepted on the command line and listed in help. They don't get their own short form; the metavar still comes from the primary name.

```rust
start_file: required(Arg::new().short('s').aliases(&["sf"]).desc("Start file"))
// -s x | --start-file x | --sf x
// help: -s, --start-file, --sf <START-FILE>  Start file
```

An alias that collides with another argument's name or alias is a registration panic.

**Required positional.** One slot, must be present.

```rust
output: positional(Arg::new())
// pack out.bin → out.bin
// pack         → error: missing required positional argument: output
```

**Optional positional.** One slot, may be absent.

```rust
name: positional_optional(Arg::new())
// pack out.bin myproj → Some("myproj")
// pack out.bin        → None
```

**Variadic positional.** Zero or more slots, must be declared last. Use a check after parsing to require at least one.

```rust
inputs: positional_variadic(Arg::new())
// pack out.bin a b c → [a, b, c]
// pack out.bin       → []
```

**`--` separator.** Everything after it is treated as a positional, so hyphen-prefixed tokens survive.

```rust
rest: positional_variadic(Arg::new())
// prog -- -x -y → ["-x", "-y"]
```

**Negative numbers.** Values that parse as `i64` or `f64` are accepted even though they start with `-`, no configuration needed.

```rust
offset: default(0i32, Arg::new().short('o'))
// --offset -5 → -5
```

**Other hyphen-prefixed values.** Off by default; enable per-argument.

```rust
pattern: optional(Arg::new().short('p').allow_hyphen_values())
// --pattern -file → "-file"
// without allow_hyphen_values this errors, with a hint to use --pattern=-file
```

**Type names in error messages.** By default the Rust type is used; `.type_name("x")` replaces it; `.type_name_hidden()` omits it. The effects are visible only on parse failures, so they matter for `u32` and friends, not for `String` or `PathBuf` which accept anything.

```rust
jobs: default(1u32, Arg::new().short('j').type_name("count"))
// --jobs abc
// error: invalid value 'abc' for --jobs (expected count): invalid digit found in string
```

**Validation.** Runs on the constructed struct, after parsing, in plain Rust.

```rust
if args.inputs.is_empty() {
    parser.die("at least one input file is required");
}
```

`parser.die(msg)` prints `error: {msg}` followed by a `--help` hint and exits with code 2.

## Enumerated values

Implement `Choice` for a closed set of values. The parser learns the accepted names, uses them for error messages, and lists them in `--help`:

```rust
#[derive(Debug, Clone, Copy)]
enum Color { Auto, Always, Never }

impl Choice for Color {
    const ALL: &'static [Color] = all_variants!(Color; Auto, Always, Never);

    fn name(&self) -> &'static str {
        match self {
            Color::Auto => "auto",
            Color::Always => "always",
            Color::Never => "never",
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
```

Use it like any other type:

```rust
let color = parser.default("color", Color::Auto, Arg::new().desc("Color mode"));
```

The value strings appear once, in `name()`. Everything else — parsing, the help hint, the "requires a value" error — derives from `ALL` and `name()`.

```
$ pack --color wef
error: invalid value 'wef' for --color (one of: auto, always, never)

$ pack --color
error: --color requires a value (one of: auto, always, never)

$ pack --help
      --color <COLOR>  Color mode [auto|always|never] [default: auto]
```

The variant list is written twice — once in the enum, once in `all_variants!`. The macro expands to an exhaustive `match`, so `ALL` won't build until every variant is listed:

```
error[E0004]: non-exhaustive patterns: `Color::New` not covered
  --> src/main.rs:13:36
   |
13 |     const ALL: &'static [Color] = all_variants!(Color; Auto, Always, Never);
   |                                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

The macro works for unit variants only — a variant with a payload doesn't fit the trait's model of a closed set of names.

`Choice` requires `Copy`, `Display`, and `'static`. Implementing it gives you a `ParseArg` impl for free via a blanket impl. The default `from_name` is a linear scan over `ALL`, comparing `name()`; override it with a `match` for large enums.

## Custom types

For types without a fixed value set, implement `ParseArg` directly. A newtype with validation is the typical case:

```rust
use std::{ffi::OsStr, fmt};

use miniargs::{Arg, ArgParser, ParseArg};

/// A port number. `ParseArg` rejects zero, so values that come from the
/// command line are always valid.
#[derive(Clone, Copy)]
struct Port(u16);

impl ParseArg for Port {
    fn parse_arg(s: &OsStr) -> Result<Self, String> {
        let s = s.to_str().ok_or("value is not valid UTF-8")?;
        let n: u16 = s.parse().map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("port must be greater than zero".into());
        }
        Ok(Port(n))
    }
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

Use it like any other type:

```rust
let port = parser.default(
    "port",
    Port(8080),
    Arg::new().short('p').desc("Port to listen on").type_name("port"),
);
// ...
let port: Port = port.get(&args);
```

The error string you return becomes the message after the colon:

```
$ pack --port 0
error: invalid value '0' for --port (expected port): port must be greater than zero
```

Two requirements, both only for `default`:

- **`Clone`** — the default is cloned when the argument is absent.
- **`Display`** — the default is rendered in help as `[default: 8080]`.

`required` and `optional` need neither. If you'd rather not add them, use `optional` and unwrap at the read site:

```rust
let port = parser.optional("port", Arg::new().short('p').desc("Port to listen on"));
let port: Port = port.get(&args).unwrap_or(Port(8080));
```

The help no longer shows the default, but the wrapper stays minimal.

`.type_name("port")` is optional — without it the message says `(expected Port)`, and `.type_name_hidden()` drops the `(expected ...)` clause entirely.

For external types, wrap them in a newtype and impl `ParseArg` on the wrapper. The orphan rule prevents implementing `ParseArg` on a foreign type directly.

## Non-UTF-8 correctness

`std::env::args_os()` yields `OsString`, and `miniargs` stores values as `OsString` throughout. Three `ParseArg` impls handle non-UTF-8:

- `OsString` — always succeeds, holds raw bytes.
- `PathBuf` — wraps the `OsString`, no UTF-8 conversion.
- `String` — requires valid UTF-8, errors otherwise.

Numeric and other primitive types go through `FromStr` via the `impl_parse_arg_from_str!` macro.

The one limitation: the inline `--name=<value>` form requires the whole argument to be UTF-8, because splitting an `OsStr` at `=` without `unsafe` is not possible on stable Rust. Use `--name <value>` for non-UTF-8 values. The error message says exactly this:

```
error: non-UTF-8 value in --config=…; use `--config VALUE` instead
```

## Error handling

User errors are printed to stderr and exit with code 2. Two shapes:

```
$ pack --jobs abc
error: invalid value 'abc' for --jobs (expected count): invalid digit found in string
Run with --help for usage.

$ pack out.bin
error: at least one input file is required
Run with --help for usage.
```

Structural errors (unknown option, missing required, unexpected positional) print the full usage text instead:

```
$ pack --bogus
error: unknown option --bogus

Usage: pack [OPTIONS] <output> [inputs...]
...
```

Programmer errors (calling `get` on an unregistered key, registering a duplicate argument, declaring a required positional after an optional one) panic with a descriptive message. These are bugs in your code, not user input, and only appear during development.

## Subcommands

Dispatch on `argv[1]` with `Dispatcher`. Each command gets its own `ArgParser` and builds its own struct; the dispatcher just routes and generates the top-level help.

```rust
use std::ffi::OsString;

use miniargs::{parse_args, Arg, ArgParser, Dispatcher};

fn main() {
    let mut dispatcher = Dispatcher::new("myprog");
    dispatcher.command("add", cmd_add, "Add files");
    dispatcher.command("remove", cmd_remove, "Remove files");
    dispatcher.help_command("help", "Print help");
    dispatcher.run(std::env::args_os());
}

fn cmd_add(prog: &str, argv: &[OsString]) {
    let mut parser = ArgParser::new(format!("{prog} add"));
    let args = parse_args!(parser, argv; AddArgs {
        // ...
    });
    // ...
}
```

`argv[0]` is the command name, which the handler's parser skips as `argv[0]`, so `parse_args!` works unchanged.

- `myprog --help` prints the dispatcher's help, exits 0.
- `myprog add --help` prints the subcommand's help, exits 0.
- `myprog bogus` prints an error and the dispatcher's help, exits 2.

`help_command` is optional. Without it, `-h`, `--help`, `--`, and no arguments still print help; only the word `myprog help` stops working. When registered, the command appears in the help output alongside the others.

See `examples/subcommands.rs`.

## Without the macro

`parse_args!` is the recommended path, but everything works without it. Call the same methods directly, then read each key:

```rust
let mut parser = ArgParser::new("pack");

let host = parser.required("host", Arg::new().short('H').desc("Host to bind"));
let verbose = parser.flag("verbose", Arg::new().short('v').desc("Verbose output"));
let output = parser.positional("output", Arg::new().desc("Output file"));

let args = parser.parse(std::env::args_os());

let host: String = host.get(&args);
let verbose: bool = verbose.get(&args);
let output: PathBuf = output.get(&args);
```

When you don't use the macro, the name is the first argument to each method, and the type comes from the `let` annotation at the read site: `let host: String = host.get(&args);`. Call `miniargs::kebab("file_name")` if you want the same `_` to `-` conversion the macro applies.

See `examples/manual.rs`.

## Typed keys

Each registration method returns a key whose type parameter records the value type:

```rust
let jobs = parser.default("jobs", 1u32, Arg::new());
// jobs: DefaultKey<u32>

let host = parser.required::<String>("host", Arg::new());
// host: RequiredKey<String>

let verbose = parser.flag("verbose", Arg::new());
// verbose: FlagKey
```

Reading a value is a single call with no string lookup:

```rust
let jobs: u32 = jobs.get(&args);
let host: String = host.get(&args);
let verbose: bool = verbose.get(&args);
```

Because the type and presence policy live on the key, the compiler catches three classes of mistake at build time:

- Asking for a flag with `get` on a value key, or vice versa (different types).
- Reading a `u32` argument as a `String` (the key's `get` returns `u32`; assigning to a `String` fails).
- A typo in the variable name (`hots.get(&args)` is `cannot find value`).

The only thing that isn't checked at compile time is registration, and even that is closed off: the only way to construct a key is via a registration method, which registers the argument as a side effect.

## What's not supported

- Multi-level subcommands — the dispatcher handles one dispatch; nesting is manual.
- Shell completions.
- Config-file integration — parse your config file separately and merge.

## License

MIT.