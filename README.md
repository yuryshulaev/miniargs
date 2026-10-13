# miniargs

A small, zero-dependency argument parser for Rust.

## Why

`clap` is quite heavy and its derive API depends on `syn`, which adds seconds to a clean build. `miniargs` has zero dependencies and no proc macros.

## Design goals

1. **No proc macros, no `syn`.** Fast compile, zero dependencies.
2. **No `unsafe`, zero or minimal platform-specific code.**
3. **Convenient.** Typed keys, DRY declarations, auto help.
4. **Small.** One file.
5. **Correct on non-UTF-8 input.** `PathBuf` and `OsString` accept arbitrary bytes, while `String` requires UTF-8.

## Installation

```
cargo add miniargs
```

## Example

```rust
use std::path::PathBuf;

use miniargs::{arg, args, validation::non_empty_vec, ArgParser};

args! {
    #[derive(Debug)]
    struct Args {
        target: String = required(arg().short('t').desc("Build target")),
        jobs: u32 = default(1, arg().short('j').desc("Parallel jobs").type_name("count")),
        // `project_name` becomes `--project-name` on the command line (all '_' become '-').
        project_name: Option<String> = optional(arg().desc("Project name")),
        log: Option<PathBuf> = optional(arg().desc("Log file path")),
        verbose: bool = flag(arg().short('v').desc("Verbose output")),
        include: Vec<PathBuf> = repeated(arg().short('I').desc("Add include directory")),
        output: PathBuf = positional(arg().desc("Output file")),
        inputs: Vec<PathBuf> = positional_variadic(arg().desc("Input files"))
	        .validate(non_empty_vec),
    }
}

fn main() {
    let mut parser = ArgParser::new("pack");
    parser.set_version(env!("CARGO_PKG_VERSION"));
    let args = Args::parse(&mut parser);

    if args.include.len() > 3 {
        parser.die(format!("at most 3 --include, got {}", args.include.len()));
    }

    println!("{args:#?}");
}
```

Run with `--help` to see the generated usage text:

```
…/miniargs $ cargo r -q --example args -- --help
Usage: pack [OPTIONS] <output> [inputs...]

Arguments:
  <output>                           Output file [required]
  <inputs>...                        Input files

Options:
  -t, --target <TARGET>              Build target [required]
  -j, --jobs <JOBS>                  Parallel jobs [default: 1]
      --project-name <PROJECT-NAME>  Project name
      --log <LOG>                    Log file path
  -v, --verbose                      Verbose output
  -I, --include <INCLUDE>            Add include directory
  -h, --help                         Print help
  -V, --version                      Print version
```

## Argument shapes, by example

Each shape below is shown in `parse_args!` form (`field: builder(...)`). In `args!`, the same builder goes after a type: `field: Type = builder(...)`.

**Required named option.** Missing it is an error.

```rust
target: required(arg().short('t'))
// --target x
// -t x
```

**Defaulted named option.** Absent means the default.

```rust
jobs: default(1u32, arg().short('j'))
// omitted      → 1
// --jobs 8     → 8
```

**Optional named option.** Absent means `None`.

```rust
log: optional(arg().short('l'))
// omitted   → None
// --log x   → Some(PathBuf::from("x"))
```

**Boolean flag.** Present means `true`.

```rust
verbose: flag(arg().short('v'))
// omitted    → false
// -v         → true
// --verbose  → true
```

**Repeated option.** Every occurrence accumulates.

```rust
include: repeated(arg().short('I'))
// -Ia -I b --include=c → [a, b, c]
// omitted              → []
```

**Aliases.** Alternate long names, accepted on the command line and listed in help. They don't get their own short form; the metavar still comes from the primary name.

```rust
start_file: required(arg().short('s').aliases(&["sf"]).desc("Start file"))
// -s x | --start-file x | --sf x
// help: -s, --start-file, --sf <START-FILE>  Start file
```

An alias that collides with another argument's name or alias is a registration panic.

**Required positional.** One slot, must be present.

```rust
output: positional(arg())
// pack out.bin → out.bin
// pack         → error: missing required positional argument: output
```

**Optional positional.** One slot, may be absent.

```rust
name: positional_optional(arg())
// pack out.bin myproj → Some("myproj")
// pack out.bin        → None
```

**Variadic positional.** Zero or more slots, must be declared last. Use a check after parsing to require at least one.

```rust
inputs: positional_variadic(arg())
// pack out.bin a b c → [a, b, c]
// pack out.bin       → []
```

**`--` separator.** Everything after it is treated as a positional, so hyphen-prefixed tokens survive.

```rust
rest: positional_variadic(arg())
// prog -- -x -y → ["-x", "-y"]
```

**Negative numbers.** Values that parse as `i64` or `f64` are accepted even though they start with `-`, no configuration needed.

```rust
offset: default(0i32, arg().short('o'))
// --offset -5 → -5
```

**Other hyphen-prefixed values.** Off by default; enable per-argument.

```rust
pattern: optional(arg().short('p').allow_hyphen_values())
// --pattern -file → "-file"
// without allow_hyphen_values this errors, with a hint to use --pattern=-file
```

**Type names in error messages.** By default the Rust type is used; `.type_name("x")` replaces it; `.type_name_hidden()` omits it. The effects are visible only on parse failures, so they matter for `u32` and friends, not for `String` or `PathBuf` which accept anything.

```rust
jobs: default(1u32, arg().short('j').type_name("count"))
// --jobs abc
// error: invalid value 'abc' for --jobs (expected count): invalid digit found in string
```

## Validation

Two kinds of checks: per-argument and cross-argument.

**Per-argument** checks live on the key. Chain `.validate(f)`, where `f` is a `fn(&T) -> Result<(), String>` and `T` is the value type — `Vec<T>` for a repeated argument. Multiple validators accumulate; all run in order and the first failure exits with code 2.

```rust
jobs: default(1, arg().short('j')).validate(positive).validate(odd),
```

```rust
$ pack --jobs 0
error: invalid --jobs: must be positive
Run with --help for usage.
```

A closure works the same way, which is useful when you want a couple of checks in one place:

```rust
jobs: default(1, arg().short('j')).validate(|v| {
    positive(v)?;
    odd(v)?;
    Ok(())
}),
```

The `validation` module provides `positive`, `non_empty`, `non_empty_vec`, and `require` (a helper for writing your own). See `examples/validation.rs`.

**Cross-argument** checks stay in plain Rust after parsing. Anything that depends on more than one argument can't live on a key:

```rust
if args.config.is_some() && args.env.is_some() {
    parser.die("--config and --env are mutually exclusive");
}
```

`parser.die(msg)` prints `error: {msg}` followed by a `--help` hint and exits with code 2.

## The `args!` macro

The example at the top uses `args!`. Each field is `name: Type = builder(...)`; the name becomes the CLI name, kebab-cased.

Two constructors are generated. `parse(&mut parser)` reads `std::env::args_os()`; `parse_from(&mut parser, argv)` reads an explicit iterator for tests, and `argv` must start with the program name.

```rust
let argv = vec![OsString::from("pack"), OsString::from("-v")];
let mut parser = ArgParser::new("pack");
let args = PackArgs::parse_from(&mut parser, argv);
```

## The `parse_args!` macro

If your struct is defined separately, use `parse_args!`:

```rust
let args = parse_args!(parser; Args {
    file_name: required(arg().short('f').desc("File to process")),
});
```

This expands to:

```rust
let args = {
    let file_name = parser.required(
        kebab("file_name"),
        arg().short('f').desc("File to process"),
    );
    let args = parser.parse(std::env::args_os());
    Args { file_name: file_name.get(&args) }
};
```

The struct field type drives the parser type (`file_name: String` instantiates `required::<String>`), so nothing is written twice. The parser is passed explicitly because `macro_rules!` hygiene prevents the macro body from referring to a caller binding by name.

For tests, use the two-argument form with an explicit argv:

```rust
let argv = vec![OsString::from("pack"), OsString::from("-v"), OsString::from("out.bin")];
let args = parse_args!(parser, argv; Args {
    verbose: flag(arg().short('v')),
    output: positional(arg()),
});
```

## Without macros

Everything works without either macro. Call the same methods directly, then read each key:

```rust
let mut parser = ArgParser::new("pack");

let host = parser.required("host", arg().short('H').desc("Host to bind"));
let verbose = parser.flag("verbose", arg().short('v').desc("Verbose output"));
let output = parser.positional("output", arg().desc("Output file"));

let args = parser.parse(std::env::args_os());

let host: String = host.get(&args);
let verbose: bool = verbose.get(&args);
let output: PathBuf = output.get(&args);
```

See `examples/manual.rs` for a full example.

## Customizing help

Three setters add program-level text to `--help`:

```rust
let mut parser = ArgParser::new("pack");
parser.set_version(env!("CARGO_PKG_VERSION"));
parser.set_about("Build a project and collect its artifacts.");
parser.set_footer("Report bugs at https://github.com/you/pack/issues");
```

`set_version` enables `--version`/`-V` and adds the line to the options. `set_about` prints before the usage line, `set_footer` after the options.

```
Build a project and collect its artifacts.

Usage: pack [OPTIONS] <output> [inputs...]

Arguments:
  <output>       Output file [required]
  <inputs>...    Input files

Options:
  -t, --target <TARGET>  Build target [required]
  -h, --help             Print help
  -V, --version          Print version

Report bugs at https://github.com/you/pack/issues
```

All three accept `impl Into<Cow<'static, str>>`, so `&'static str` literals and `format!(...)` both work. Multi-line text works with `\n`, and nothing is reflowed — what you write is what appears.

## Enumerated values

`enum_choice!` defines an enum and implements `Choice` for it. The parser learns the accepted names, uses them for error messages, and lists them in `--help`:

```rust
enum_choice! {
    #[derive(Debug, Clone, Copy)]
    enum Color {
        Auto => "auto",
        Always => "always",
        Never => "never",
    }
}
```

Use it like any other type:

```rust
let color = parser.default("color", Color::Auto, arg().desc("Color mode"));
```

The value strings appear once, in the macro call. Everything else — the enum, `Choice`, `Display`, the help hint, the "requires a value" error — derives from that one list.

```
$ pack --color wef
error: invalid value 'wef' for --color (one of: auto, always, never)

$ pack --color
error: --color requires a value (one of: auto, always, never)

$ pack --help
      --color <COLOR>  Color mode [auto|always|never] [default: auto]
```

If the enum already exists, define it yourself and use `impl_choice!`:

```rust
#[derive(Debug, Clone, Copy)]
enum Color { Auto, Always, Never }

impl_choice!(Color;
    Auto => "auto",
    Always => "always",
    Never => "never",
);

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
```

Here the variants are listed twice — once in the enum, once in the macro — so a variant added to the enum but not the macro is a compile error:

```
error[E0004]: non-exhaustive patterns: `Color::New` not covered
  --> src/main.rs:5:9
   |
 5 |         Auto => "auto",
   |         ^^^^ pattern `Color::New` not covered
```

Both macros work for unit variants only — a variant with a payload doesn't fit the trait's model of a closed set of names.

`Choice` requires `Copy`, `Display`, and `'static`. Implementing it gives you a `ParseArg` impl for free via a blanket impl. The default `from_name` is a linear scan over `ALL`, comparing `name()`; override it with a `match` for large enums.

## Custom types

For types without a fixed value set, implement `ParseArg` directly. A newtype with validation is the typical case:

```rust
use std::{ffi::OsStr, fmt};

use miniargs::{arg, ArgParser, ParseArg};

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
    arg().short('p').desc("Port to listen on").type_name("port"),
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
let port = parser.optional("port", arg().short('p').desc("Port to listen on"));
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

One limitation: the inline `--name=<value>` form requires the whole argument to be UTF-8. Splitting an `OsStr` at `=` requires either `unsafe` or platform-specific APIs, and miniargs avoids both. Use `--name <value>` for non-UTF-8 values. The error message says exactly this:

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

use miniargs::{arg, args, ArgParser, Dispatcher};

args! {
    #[derive(Debug)]
    struct AddArgs {
        collection_name: String = required(arg().short('n').desc("Collection")),
        force: bool = flag(arg().short('f').desc("Overwrite existing entries")),
        files: Vec<PathBuf> = positional_variadic(arg().desc("Files to add")),
    }
}

fn main() {
    let mut dispatcher = Dispatcher::new("myprog");
    dispatcher.command("add", cmd_add, "Add files");
    dispatcher.command("remove", cmd_remove, "Remove files");
    dispatcher.help_command("help", "Print help");
	dispatcher.run();
}

fn cmd_add(prog: &str, argv: &[OsString]) {
    let mut parser = ArgParser::new(format!("{prog} add"));
    let args = AddArgs::parse_from(&mut parser, argv);
    // ...
}
```

`run()` reads `std::env::args_os()`. `run_with(argv)` takes an explicit argv (useful for testing).

- `myprog --help` prints the dispatcher's help, exits 0.
- `myprog add --help` prints the subcommand's help, exits 0.
- `myprog bogus` prints an error and the dispatcher's help, exits 2.

`help_command` is optional. Without it, `-h`, `--help`, `--`, and no arguments still print help; only the word `myprog help` stops working. When registered, the command appears in the help output alongside the others.

See `examples/subcommands.rs`.

## Typed keys

Each registration method returns a key whose type parameter records the value type:

```rust
let jobs = parser.default("jobs", 1u32, arg());
// jobs: DefaultKey<u32>

let host = parser.required::<String>("host", arg());
// host: RequiredKey<String>

let verbose = parser.flag("verbose", arg());
// verbose: FlagKey
```

Reading a value is a single call with no string lookup:

```rust
let jobs: u32 = jobs.get(&args);
let host: String = host.get(&args);
let verbose: bool = verbose.get(&args);
```

Because the type and presence policy live on the key, the compiler catches three classes of mistakes at build time:

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