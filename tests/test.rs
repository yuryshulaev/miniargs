use std::{
	borrow::Cow,
	ffi::OsString,
	fmt::{self, Display},
	path::PathBuf,
};

use miniargs::*;

fn argv(parts: &[&str]) -> Vec<OsString> {
	std::iter::once(OsString::from("test"))
		.chain(parts.iter().map(|s| OsString::from(*s)))
		.collect()
}

struct Fixture {
	parser: ArgParser,
	host: RequiredKey<String>,
	port: DefaultKey<u16>,
	config: OptionalKey<PathBuf>,
	verbose: FlagKey,
}

fn fixture() -> Fixture {
	let mut parser = ArgParser::new("test");
	let host = parser
		.required::<String>("host", Arg { short: 'H', desc: "Host to bind", ..Default::default() });
	let port = parser.default(
		"port",
		8080u16,
		Arg { short: 'p', desc: "Port to listen on", ..Default::default() },
	);
	let config = parser.optional::<PathBuf>(
		"config",
		Arg { short: 'c', desc: "Path to config file", ..Default::default() },
	);
	let verbose =
		parser.flag("verbose", Arg { short: 'v', desc: "Verbose output", ..Default::default() });
	Fixture { parser, host, port, config, verbose }
}

fn parse_ok<'a>(f: &'a Fixture, args: &[&str]) -> ParsedArgs<'a> {
	f.parser.try_parse(argv(args)).expect("expected Ok")
}

fn parse_err(f: &Fixture, args: &[&str]) -> String {
	parse_err_of(&f.parser, args)
}

fn parse_err_of(p: &ArgParser, args: &[&str]) -> String {
	match p.try_parse(argv(args)) {
		Ok(_) => panic!("expected Err"),
		Err(ParseError::User(msg)) => msg,
		Err(other) => panic!("expected User error, got {other:?}"),
	}
}

// ---------------------------------------------------------------------------
// Long options
// ---------------------------------------------------------------------------

#[test]
fn long_separate_value() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "example.com"]);
	assert_eq!(f.host.get(&a), "example.com");
}

#[test]
fn long_inline_value() {
	let f = fixture();
	let a = parse_ok(&f, &["--host=example.com"]);
	assert_eq!(f.host.get(&a), "example.com");
}

#[test]
fn long_missing_value_errors() {
	let f = fixture();
	let e = parse_err(&f, &["--host"]);
	assert!(e.contains("--host") && e.contains("requires a value"), "{e}");
}

#[test]
fn long_unknown_errors() {
	let f = fixture();
	let e = parse_err(&f, &["--nope"]);
	assert!(e.contains("--nope"), "{e}");
}

#[test]
fn long_value_on_flag_errors() {
	let f = fixture();
	let e = parse_err(&f, &["--verbose=yes"]);
	assert!(e.contains("does not take a value"), "{e}");
}

// ---------------------------------------------------------------------------
// Aliases
// ---------------------------------------------------------------------------

#[test]
fn alias_resolves_to_same_value() {
	let mut p = ArgParser::new("test");
	let file = p.required::<String>("start-file", Arg::new().aliases(&["sf"]));
	let a = p.try_parse(argv(&["--sf", "x"])).unwrap();
	assert_eq!(file.get(&a), "x");
}

#[test]
fn primary_name_still_works_with_alias() {
	let mut p = ArgParser::new("test");
	let file = p.required::<String>("start-file", Arg::new().aliases(&["sf"]));
	let a = p.try_parse(argv(&["--start-file", "x"])).unwrap();
	assert_eq!(file.get(&a), "x");
}

#[test]
fn aliases_appear_in_help() {
	let mut p = ArgParser::new("test");
	let _file = p.required::<String>("start-file", Arg::new().short('s').aliases(&["sf"]));
	let h = p.help();
	assert!(h.contains("-s, --start-file, --sf <START-FILE>"), "{h}");
}

#[test]
#[should_panic(expected = "collides with an existing argument")]
fn alias_collision_panics() {
	let mut p = ArgParser::new("test");
	p.flag("verbose", Arg::new());
	p.flag("v", Arg::new().aliases(&["verbose"]));
}

// ---------------------------------------------------------------------------
// Short options
// ---------------------------------------------------------------------------

#[test]
fn short_separate_value() {
	let f = fixture();
	let a = parse_ok(&f, &["-H", "example.com"]);
	assert_eq!(f.host.get(&a), "example.com");
}

#[test]
fn short_glued_value() {
	let f = fixture();
	let a = parse_ok(&f, &["-Hexample.com"]);
	assert_eq!(f.host.get(&a), "example.com");
}

#[test]
fn short_equals_value() {
	let f = fixture();
	let a = parse_ok(&f, &["-H=example.com"]);
	assert_eq!(f.host.get(&a), "example.com");
}

#[test]
fn short_unknown_errors() {
	let f = fixture();
	let e = parse_err(&f, &["-z"]);
	assert!(e.contains("-z"), "{e}");
}

#[test]
fn short_cluster_flag_then_value() {
	let f = fixture();
	let a = parse_ok(&f, &["-vHexample.com"]);
	assert!(f.verbose.get(&a));
	assert_eq!(f.host.get(&a), "example.com");
}

#[test]
fn short_flag_with_equals_errors() {
	let f = fixture();
	let e = parse_err(&f, &["-v=yes"]);
	assert!(e.contains("--verbose"), "{e}");
	assert!(e.contains("does not take a value"), "{e}");
}

#[test]
fn short_dash_is_not_matchable() {
	let mut p = ArgParser::new("test");
	let _v = p.flag("verbose", Arg { short: 'v', ..Default::default() });
	// `-v-` must not match a no-short def via the `-` sentinel.
	let e = parse_err_of(&p, &["-v-"]);
	assert!(e.contains("unknown option"), "{e}");
}

// ---------------------------------------------------------------------------
// Presence policies
// ---------------------------------------------------------------------------

#[test]
fn default_used_when_absent() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "x"]);
	assert_eq!(f.port.get(&a), 8080);
}

#[test]
fn default_overridden() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "x", "--port", "9000"]);
	assert_eq!(f.port.get(&a), 9000);
}

#[test]
fn optional_absent_is_none() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "x"]);
	assert_eq!(f.config.get(&a), None);
}

#[test]
fn optional_present_is_some() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "x", "--config", "/etc/x.toml"]);
	assert_eq!(f.config.get(&a), Some(PathBuf::from("/etc/x.toml")));
}

#[test]
fn flag_absent_is_false() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "x"]);
	assert!(!f.verbose.get(&a));
}

#[test]
fn flag_present_is_true() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "x", "--verbose"]);
	assert!(f.verbose.get(&a));
}

#[test]
fn missing_required_errors() {
	let f = fixture();
	let e = parse_err(&f, &[]);
	assert!(e.contains("host") && e.contains("required"), "{e}");
}

#[test]
fn duplicate_single_value_errors() {
	let f = fixture();
	let e = parse_err(&f, &["--host=a", "--host=b"]);
	assert!(e.contains("more than once"), "{e}");
}

// ---------------------------------------------------------------------------
// Hidden arguments
// ---------------------------------------------------------------------------

#[test]
fn hidden_value_is_accepted_and_unlisted() {
	let mut p = ArgParser::new("test");
	let backend = p.hidden::<String>("backend", Arg::new());
	let a = p.try_parse(argv(&["--backend", "x"])).unwrap();
	assert_eq!(backend.get(&a), Some("x".to_string()));
	assert!(!p.help().contains("backend"));
}

#[test]
fn hidden_flag_is_accepted_and_unlisted() {
	let mut p = ArgParser::new("test");
	let dump = p.hidden_flag("dump-tokens", Arg::new());
	let a = p.try_parse(argv(&["--dump-tokens"])).unwrap();
	assert!(dump.get(&a));
	assert!(!p.help().contains("dump"));
}

// ---------------------------------------------------------------------------
// Choice / enumerated values
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Color {
	Auto,
	Always,
	Never,
}

impl Choice for Color {
	const ALL: &'static [Color] = &[Color::Auto, Color::Always, Color::Never];

	fn name(&self) -> &'static str {
		match self {
			Color::Auto => "auto",
			Color::Always => "always",
			Color::Never => "never",
		}
	}
}

impl Display for Color {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.name())
	}
}

#[test]
fn choice_values_lists_all() {
	assert_eq!(Color::values(), vec!["auto", "always", "never"]);
}

#[test]
fn choice_parses_valid_value() {
	let mut p = ArgParser::new("test");
	let color = p.default("color", Color::Auto, Arg::new());
	let a = p.try_parse(argv(&["--color", "always"])).unwrap();
	assert_eq!(color.get(&a), Color::Always);
}

#[test]
fn choice_default_when_absent() {
	let mut p = ArgParser::new("test");
	let color = p.default("color", Color::Auto, Arg::new());
	let a = p.try_parse(argv(&[])).unwrap();
	assert_eq!(color.get(&a), Color::Auto);
}

#[test]
fn choice_requires_value_lists_options() {
	let mut p = ArgParser::new("test");
	let _color = p.default("color", Color::Auto, Arg::new());
	let e = parse_err_of(&p, &["--color"]);
	assert!(e.contains("one of: auto, always, never"), "{e}");
}

#[test]
fn non_choice_requires_value_has_no_list() {
	let mut p = ArgParser::new("test");
	let _jobs = p.default("jobs", 1u32, Arg::new());
	let e = parse_err_of(&p, &["--jobs"]);
	assert!(e.contains("--jobs requires a value"), "{e}");
	assert!(!e.contains("one of"), "{e}");
}

#[test]
fn enum_choice_macro_produces_working_choice() {
	miniargs::enum_choice! {
		#[derive(Debug, Clone, Copy, PartialEq)]
		enum Size { Small => "small", Large => "large" }
	}
	assert_eq!(Size::values(), vec!["small", "large"]);
	assert_eq!(Size::Small.to_string(), "small");

	let mut p = ArgParser::new("test");
	let s = p.default("size", Size::Small, Arg::new());
	let a = p.try_parse(argv(&["--size", "large"])).unwrap();
	assert_eq!(s.get(&a), Size::Large);
}

// ---------------------------------------------------------------------------
// Help and version
// ---------------------------------------------------------------------------

#[test]
fn long_help() {
	let f = fixture();
	assert!(matches!(f.parser.try_parse(argv(&["--help"])), Err(ParseError::Help)));
}

#[test]
fn short_help() {
	let f = fixture();
	assert!(matches!(f.parser.try_parse(argv(&["-h"])), Err(ParseError::Help)));
}

#[test]
fn help_inside_short_cluster() {
	let f = fixture();
	assert!(matches!(f.parser.try_parse(argv(&["-vh"])), Err(ParseError::Help)));
}

#[test]
fn version_long() {
	let mut p = ArgParser::new("test");
	p.set_version("1.2.3");
	assert!(matches!(p.try_parse(argv(&["--version"])), Err(ParseError::Version)));
}

#[test]
fn version_short() {
	let mut p = ArgParser::new("test");
	p.set_version("1.2.3");
	assert!(matches!(p.try_parse(argv(&["-V"])), Err(ParseError::Version)));
}

// ---------------------------------------------------------------------------
// Separator and positionals
// ---------------------------------------------------------------------------

#[test]
fn separator_alone_is_ok() {
	let f = fixture();
	let a = parse_ok(&f, &["--host", "x", "--"]);
	assert_eq!(f.host.get(&a), "x");
}

#[test]
fn dash_is_a_positional() {
	let mut p = ArgParser::new("test");
	let input = p.positional::<String>("input", Arg { ..Default::default() });
	let a = p.try_parse(argv(&["-"])).unwrap();
	assert_eq!(input.get(&a), "-");
}

#[test]
fn dash_is_a_value() {
	let mut p = ArgParser::new("test");
	let out = p.required::<String>("out", Arg { short: 'o', ..Default::default() });
	let a = p.try_parse(argv(&["-o", "-"])).unwrap();
	assert_eq!(out.get(&a), "-");
}

#[test]
fn positional_without_variadic_errors() {
	let f = fixture();
	let e = parse_err(&f, &["--host", "x", "junk"]);
	assert!(e.contains("junk"), "{e}");
}

#[test]
fn positional_after_separator_errors() {
	let f = fixture();
	let e = parse_err(&f, &["--host", "x", "--", "junk"]);
	assert!(e.contains("junk"), "{e}");
}

// ---------------------------------------------------------------------------
// Hyphen-prefixed values
// ---------------------------------------------------------------------------

#[test]
fn negative_integer_accepted_without_allow_hyphen() {
	let mut p = ArgParser::new("test");
	let n = p.default("n", 0i32, Arg { ..Default::default() });
	let a = p.try_parse(argv(&["--n", "-5"])).unwrap();
	assert_eq!(n.get(&a), -5);
}

#[test]
fn negative_float_accepted_without_allow_hyphen() {
	let mut p = ArgParser::new("test");
	let x = p.default("x", 0.0f64, Arg { ..Default::default() });
	let a = p.try_parse(argv(&["--x", "-3.14"])).unwrap();
	assert!((x.get(&a) - (-3.14)).abs() < 1e-12);
}

#[test]
fn allow_hyphen_consumes_non_numeric_value() {
	let mut p = ArgParser::new("test");
	let pat = p.optional::<String>(
		"pat",
		Arg { short: 'p', allow_hyphen_values: true, ..Default::default() },
	);
	let a = p.try_parse(argv(&["--pat", "-file"])).unwrap();
	assert_eq!(pat.get(&a), Some("-file".to_string()));
}

#[test]
fn disallowed_hyphen_value_errors() {
	let f = fixture();
	let e = parse_err(&f, &["--host", "x", "--port", "-abc"]);
	assert!(e.contains("--port"), "{e}");
}

#[test]
fn inline_hyphen_value_passes() {
	let f = fixture();
	let _ = parse_ok(&f, &["--host", "x", "--port=-5"]);
}

#[test]
fn consume_value_rejects_separator() {
	let f = fixture();
	let e = parse_err(&f, &["--host", "--"]);
	assert!(e.contains("--host"), "{e}");
}

// ---------------------------------------------------------------------------
// Positionals
// ---------------------------------------------------------------------------

type Positionals = (ArgParser, RequiredKey<PathBuf>, OptionalKey<PathBuf>, RepeatedKey<PathBuf>);

fn with_positionals() -> Positionals {
	let mut p = ArgParser::new("test");
	let out = p.positional::<PathBuf>("output", Arg { ..Default::default() });
	let name = p.positional_optional::<PathBuf>("name", Arg { ..Default::default() });
	let rest = p.positional_variadic::<PathBuf>("rest", Arg { ..Default::default() });
	(p, out, name, rest)
}

#[test]
fn positional_required_provided() {
	let (p, out, _, _) = with_positionals();
	let a = p.try_parse(argv(&["out.bin"])).unwrap();
	assert_eq!(out.get(&a), PathBuf::from("out.bin"));
}

#[test]
fn positional_required_missing_errors() {
	let (p, _, _, _) = with_positionals();
	let e = parse_err_of(&p, &[]);
	assert!(e.contains("output"), "{e}");
}

#[test]
fn positional_optional_absent() {
	let (p, _, name, _) = with_positionals();
	let a = p.try_parse(argv(&["out.bin"])).unwrap();
	assert_eq!(name.get(&a), None);
}

#[test]
fn positional_optional_present() {
	let (p, _, name, _) = with_positionals();
	let a = p.try_parse(argv(&["out.bin", "myproj"])).unwrap();
	assert_eq!(name.get(&a), Some(PathBuf::from("myproj")));
}

#[test]
fn variadic_collects_rest() {
	let (p, out, _, rest) = with_positionals();
	let a = p.try_parse(argv(&["out.bin", "name", "a", "b", "c"])).unwrap();
	assert_eq!(out.get(&a), PathBuf::from("out.bin"));
	assert_eq!(rest.get(&a), vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]);
}

#[test]
fn variadic_empty() {
	let (p, _, _, rest) = with_positionals();
	let a = p.try_parse(argv(&["out.bin"])).unwrap();
	assert!(rest.get(&a).is_empty());
	assert_eq!(rest.len(&a), 0);
}

#[test]
fn variadic_interleaved_with_options() {
	let mut p = ArgParser::new("test");
	let v = p.flag("verbose", Arg { short: 'v', ..Default::default() });
	let out = p.positional::<PathBuf>("output", Arg { ..Default::default() });
	let rest = p.positional_variadic::<PathBuf>("rest", Arg { ..Default::default() });
	let a = p.try_parse(argv(&["out.bin", "-v", "a", "b"])).unwrap();
	assert!(v.get(&a));
	assert_eq!(out.get(&a), PathBuf::from("out.bin"));
	assert_eq!(rest.get(&a), vec![PathBuf::from("a"), PathBuf::from("b")]);
}

#[test]
fn variadic_keeps_hyphen_values_after_separator() {
	let mut p = ArgParser::new("test");
	let rest = p.positional_variadic::<String>("rest", Arg { ..Default::default() });
	let a = p.try_parse(argv(&["--", "-x", "y"])).unwrap();
	assert_eq!(rest.get(&a), vec!["-x".to_string(), "y".to_string()]);
}

// ---------------------------------------------------------------------------
// Repeated named options
// ---------------------------------------------------------------------------

#[test]
fn repeated_accumulates() {
	let mut p = ArgParser::new("test");
	let inc = p.repeated::<PathBuf>("include", Arg { short: 'I', ..Default::default() });
	let a = p.try_parse(argv(&["-Ia", "-I", "b", "--include=c"])).unwrap();
	assert_eq!(inc.get(&a), vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]);
}

#[test]
fn repeated_absent_is_empty() {
	let mut p = ArgParser::new("test");
	let inc = p.repeated::<PathBuf>("include", Arg { ..Default::default() });
	let a = p.try_parse(argv(&[])).unwrap();
	assert!(inc.get(&a).is_empty());
}

// ---------------------------------------------------------------------------
// Registration panics
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "argument name must not be empty")]
fn empty_name_panics() {
	let mut p = ArgParser::new("test");
	p.flag("", Arg { ..Default::default() });
}

#[test]
#[should_panic(expected = "duplicate argument name")]
fn duplicate_name_panics() {
	let mut p = ArgParser::new("test");
	p.flag("v", Arg { ..Default::default() });
	p.flag("v", Arg { ..Default::default() });
}

#[test]
#[should_panic(expected = "duplicate short option")]
fn duplicate_short_panics() {
	let mut p = ArgParser::new("test");
	p.flag("a", Arg { short: 'x', ..Default::default() });
	p.flag("b", Arg { short: 'x', ..Default::default() });
}

#[test]
#[should_panic(expected = "positional output must not have a short")]
fn positional_with_short_panics() {
	let mut p = ArgParser::new("test");
	p.positional::<String>("output", Arg { short: 'o', ..Default::default() });
}

#[test]
#[should_panic(expected = "only one variadic positional")]
fn second_variadic_panics() {
	let mut p = ArgParser::new("test");
	p.positional_variadic::<String>("a", Arg { ..Default::default() });
	p.positional_variadic::<String>("b", Arg { ..Default::default() });
}

#[test]
#[should_panic(expected = "cannot declare a positional after a variadic")]
fn positional_after_variadic_panics() {
	let mut p = ArgParser::new("test");
	p.positional_variadic::<String>("rest", Arg { ..Default::default() });
	p.positional::<String>("later", Arg { ..Default::default() });
}

#[test]
#[should_panic(expected = "required positionals must be declared before optional")]
fn required_after_optional_panics() {
	let mut p = ArgParser::new("test");
	p.positional_optional::<String>("out", Arg { ..Default::default() });
	p.positional::<String>("in", Arg { ..Default::default() });
}

// ---------------------------------------------------------------------------
// Help output
// ---------------------------------------------------------------------------

#[test]
fn help_starts_with_usage() {
	let f = fixture();
	assert!(f.parser.help().starts_with("Usage: test"));
}

#[test]
fn help_lists_options_with_short_and_long() {
	let f = fixture();
	let h = f.parser.help();
	assert!(h.contains("-H, --host <HOST>"), "{h}");
	assert!(h.contains("-p, --port <PORT>"), "{h}");
	assert!(h.contains("-c, --config <CONFIG>"), "{h}");
	assert!(h.contains("-v, --verbose"), "{h}");
}

#[test]
fn help_marks_required() {
	let f = fixture();
	let line = f.parser.help().lines().find(|l| l.contains("--host")).unwrap().to_string();
	assert!(line.contains("[required]"), "{line}");
}

#[test]
fn help_shows_default() {
	let f = fixture();
	let line = f.parser.help().lines().find(|l| l.contains("--port")).unwrap().to_string();
	assert!(line.contains("[default: 8080]"), "{line}");
}

#[test]
fn help_includes_help_line() {
	let f = fixture();
	assert!(f.parser.help().contains("-h, --help"));
}

#[test]
fn help_includes_version_when_set() {
	let mut p = ArgParser::new("test");
	p.set_version("1.0");
	assert!(p.help().contains("-V, --version"));
}

#[test]
fn help_has_arguments_section() {
	let (p, _, _, _) = with_positionals();
	let h = p.help();
	assert!(h.contains("Arguments:"), "{h}");
	assert!(h.contains("<output>"), "{h}");
	assert!(h.contains("<rest>..."), "{h}");
}

#[test]
fn help_shows_choice_values() {
	let mut p = ArgParser::new("test");
	let _color = p.default("color", Color::Auto, Arg::new().desc("Color mode"));
	let h = p.help();
	assert!(h.contains("[auto|always|never]"), "{h}");
	assert!(h.contains("[default: auto]"), "{h}");
}

#[test]
fn help_prints_about_before_usage() {
	let mut p = ArgParser::new("test");
	p.set_about("A test program.");
	let h = p.help();
	assert!(h.starts_with("A test program.\n\nUsage: test"), "{h}");
}

#[test]
fn help_prints_footer_after_options() {
	let mut p = ArgParser::new("test");
	p.set_footer("See the docs.");
	let h = p.help();
	assert!(h.trim_end().ends_with("See the docs."), "{h}");
}

#[test]
fn set_about_accepts_string() {
	let mut p = ArgParser::new("test");
	p.set_about(format!("Version {}", 1));
	assert!(p.help().starts_with("Version 1"));
}

// ---------------------------------------------------------------------------
// kebab
// ---------------------------------------------------------------------------

#[test]
fn kebab_no_underscore_borrows() {
	let s = kebab("host");
	assert!(matches!(s, Cow::Borrowed("host")));
}

#[test]
fn kebab_single_underscore() {
	assert_eq!(kebab("file_name"), "file-name");
}

#[test]
fn kebab_multiple_underscores() {
	assert_eq!(kebab("dry_run_mode"), "dry-run-mode");
}

#[test]
fn kebab_consecutive_underscores() {
	assert_eq!(kebab("a__b"), "a--b");
}

#[test]
fn kebab_leading_and_trailing() {
	assert_eq!(kebab("_x_"), "-x-");
	assert_eq!(kebab("_"), "-");
}

#[test]
fn kebab_empty() {
	let s = kebab("");
	assert!(matches!(s, Cow::Borrowed("")));
}

#[test]
fn kebab_leaves_non_ascii_alone() {
	assert_eq!(kebab("café_name"), "café-name");
	let s = kebab("café");
	assert!(matches!(s, Cow::Borrowed("café")));
}

#[test]
fn kebab_strips_raw_identifier_prefix() {
	assert_eq!(kebab("r#move"), "move");
	assert_eq!(kebab("r#type"), "type");
	assert_eq!(kebab("r#dry_run"), "dry-run");
}

// ---------------------------------------------------------------------------
// kebab integration with parse_args!
// ---------------------------------------------------------------------------

struct KebabArgs {
	file_name: String,
	dry_run: bool,
}

struct PlainArgs {
	host: String,
}

#[test]
fn macro_kebab_converts_field_name_to_cli_name() {
	let mut parser = ArgParser::new("test");
	let argv = argv(&["--file-name", "x", "--dry-run"]);
	let args = parse_args!(parser, argv; KebabArgs {
		file_name: required(Arg { ..Default::default() }),
		dry_run: flag(Arg { ..Default::default() }),
	});
	assert_eq!(args.file_name, "x");
	assert!(args.dry_run);
}

#[test]
fn macro_plain_name_unchanged() {
	let mut parser = ArgParser::new("test");
	let argv = argv(&["--host", "x"]);
	let args = parse_args!(parser, argv; PlainArgs {
		host: required(Arg { ..Default::default() }),
	});
	assert_eq!(args.host, "x");
}

#[test]
fn macro_snake_name_rejected_on_command_line() {
	let mut parser = ArgParser::new("test");
	parser.required::<String>(kebab("file_name"), Arg { ..Default::default() });
	let e = parse_err_of(&parser, &["--file_name", "x"]);
	assert!(e.contains("--file_name"), "{e}");
}

#[test]
fn kebab_name_appears_in_help() {
	let mut parser = ArgParser::new("test");
	parser.required::<String>(kebab("file_name"), Arg { ..Default::default() });
	parser.flag(kebab("dry_run"), Arg { ..Default::default() });
	let h = parser.help();
	assert!(h.contains("--file-name <FILE-NAME>"), "{h}");
	assert!(h.contains("--dry-run"), "{h}");
}

#[test]
fn kebab_positional_name_appears_in_help() {
	let mut parser = ArgParser::new("test");
	parser.positional::<String>(kebab("input_file"), Arg { ..Default::default() });
	let h = parser.help();
	assert!(h.contains("<input-file>"), "{h}");
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

fn noop(_prog: &str, _argv: &[OsString]) {}

#[test]
fn dispatcher_finds_registered_command() {
	let mut d = Dispatcher::new("test");
	d.command("add", noop, "Add things");
	d.command("remove", noop, "Remove things");
	assert!(d.find("add").is_some());
	assert!(d.find("remove").is_some());
	assert!(d.find("nope").is_none());
}

#[test]
fn dispatcher_help_lists_commands() {
	let mut d = Dispatcher::new("test");
	d.command("add", noop, "Add things");
	d.command("remove", noop, "Remove things");
	let h = d.help();
	assert!(h.contains("Usage: test <command>"), "{h}");
	assert!(h.contains("add"), "{h}");
	assert!(h.contains("Add things"), "{h}");
	assert!(h.contains("remove"), "{h}");
	assert!(h.contains("Remove things"), "{h}");
	assert!(h.contains("-h, --help"), "{h}");
}

#[test]
fn dispatcher_help_aligns_columns() {
	let mut d = Dispatcher::new("test");
	d.command("a", noop, "Short name");
	d.command("verylongcommand", noop, "Long name");
	let h = d.help();
	assert!(h.contains("  a                Short name"), "{h}");
	assert!(h.contains("  verylongcommand  Long name"), "{h}");
}

#[test]
fn dispatcher_double_dash_is_not_a_command() {
	let mut d = Dispatcher::new("test");
	d.command("add", noop, "Add things");
	// `--` is handled by the help arm in `run` and never reaches `find`.
	assert!(d.find("--").is_none());
}

#[test]
fn dispatcher_help_command_appears_in_help() {
	let mut d = Dispatcher::new("test");
	d.command("add", noop, "Add things");
	d.help_command("help", "Print this message");
	let h = d.help();
	assert!(h.contains("help"), "{h}");
	assert!(h.contains("Print this message"), "{h}");
}

// ---------------------------------------------------------------------------
// Non-UTF-8 handling (Unix only)
//
// Constructing a non-UTF-8 `OsString` from raw bytes uses
// `std::os::unix::ffi::OsStringExt::from_vec`, which doesn't exist on Windows.
// The scenario itself is also Unix-specific: Windows `OsString` is WTF-8, and
// `args_os()` never yields the raw-byte paths that motivate this design.
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn parse_err_os(p: &ArgParser, args: Vec<OsString>) -> String {
	let mut argv: Vec<OsString> = vec!["test".into()];
	argv.extend(args);
	match p.try_parse(argv) {
		Ok(_) => panic!("expected Err"),
		Err(ParseError::User(msg)) => msg,
		Err(other) => panic!("expected User error, got {other:?}"),
	}
}

#[cfg(unix)]
#[test]
fn non_utf8_inline_value_suggests_separate_form() {
	use std::os::unix::ffi::OsStringExt;

	let mut p = ArgParser::new("test");
	let _config = p.optional::<PathBuf>("config", Arg { ..Default::default() });

	let mut raw = b"--config=".to_vec();
	raw.push(0xff);
	raw.push(0xfe);
	let e = parse_err_os(&p, vec![OsString::from_vec(raw)]);

	assert!(e.contains("non-UTF-8 value in --config"), "{e}");
	assert!(e.contains("--config VALUE"), "{e}");
}

#[cfg(unix)]
#[test]
fn non_utf8_short_inline_value_names_the_flag() {
	use std::os::unix::ffi::OsStringExt;

	let mut p = ArgParser::new("test");
	let _config = p.optional::<PathBuf>("config", Arg { short: 'c', ..Default::default() });

	// -c=<non-utf8>
	let mut raw = b"-c=".to_vec();
	raw.push(0xff);
	let e = parse_err_os(&p, vec![OsString::from_vec(raw)]);

	assert!(e.contains("non-UTF-8 value in -c"), "{e}");
	assert!(e.contains("-c VALUE"), "{e}");
}

#[cfg(unix)]
#[test]
fn non_utf8_short_glued_value_has_generic_message() {
	use std::os::unix::ffi::OsStringExt;

	let mut p = ArgParser::new("test");
	let _config = p.optional::<PathBuf>("config", Arg { short: 'c', ..Default::default() });

	// -c<non-utf8>, no `=`
	let mut raw = b"-c".to_vec();
	raw.push(0xff);
	let e = parse_err_os(&p, vec![OsString::from_vec(raw)]);

	assert!(e.contains("non-UTF-8"), "{e}");
}

#[cfg(unix)]
#[test]
fn non_utf8_option_name_has_generic_message() {
	use std::os::unix::ffi::OsStringExt;

	let p = ArgParser::new("test");

	let mut raw = b"--".to_vec();
	raw.push(0xff);
	let e = parse_err_os(&p, vec![OsString::from_vec(raw)]);

	assert!(e.contains("non-UTF-8 option"), "{e}");
}

#[cfg(unix)]
#[test]
fn non_utf8_path_in_separate_form_works() {
	use std::os::unix::ffi::OsStringExt;

	let mut p = ArgParser::new("test");
	let config = p.optional::<PathBuf>("config", Arg { ..Default::default() });

	let mut path = b"/tmp/".to_vec();
	path.push(0xff);
	path.push(b'.');
	path.extend(b"toml");
	let expected = PathBuf::from(OsString::from_vec(path.clone()));

	let argv: Vec<OsString> = vec!["test".into(), "--config".into(), OsString::from_vec(path)];
	let a = p.try_parse(argv).unwrap();
	assert_eq!(config.get(&a), Some(expected));
}

#[cfg(unix)]
#[test]
fn non_utf8_path_after_separator_works() {
	use std::os::unix::ffi::OsStringExt;

	let mut p = ArgParser::new("test");
	let rest = p.positional_variadic::<PathBuf>("rest", Arg { ..Default::default() });

	let mut path = b"/tmp/".to_vec();
	path.push(0xfe);
	let expected = PathBuf::from(OsString::from_vec(path.clone()));

	let argv: Vec<OsString> = vec!["test".into(), "--".into(), OsString::from_vec(path)];
	let a = p.try_parse(argv).unwrap();
	assert_eq!(rest.get(&a), vec![expected]);
}
