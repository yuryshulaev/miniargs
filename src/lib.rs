//! miniargs: a small, zero-dependency argument parser.
//!
//! Design goals:
//!
//! 1. Fast compile, zero or minimal deps. No proc macros, no `syn`.
//! 2. Small. One file.
//! 3. Convenient. Typed keys, DRY declarations, auto help.
//! 4. Correct on non-UTF-8 input. `PathBuf` and `OsString` accept arbitrary
//!    bytes, while `String` requires UTF-8.

use std::{
	borrow::Cow,
	collections::{HashMap, HashSet},
	ffi::{OsStr, OsString},
	fmt::{self, Display},
	marker::PhantomData,
	path::PathBuf,
};

// ============================================================================
// ParseArg
// ============================================================================

pub trait ParseArg: Sized {
	fn parse_arg(s: &OsStr) -> Result<Self, String>;

	/// Accepted values, shown in "requires a value" errors and help.
	/// Empty by default; override for closed sets like enums.
	fn values() -> Vec<&'static str> {
		Vec::new()
	}
}

impl ParseArg for String {
	fn parse_arg(s: &OsStr) -> Result<Self, String> {
		s.to_str()
			.map(String::from)
			.ok_or_else(|| "value is not valid UTF-8".to_string())
	}
}

impl ParseArg for OsString {
	fn parse_arg(s: &OsStr) -> Result<Self, String> {
		Ok(s.to_os_string())
	}
}

impl ParseArg for PathBuf {
	fn parse_arg(s: &OsStr) -> Result<Self, String> {
		Ok(PathBuf::from(s))
	}
}

#[macro_export]
macro_rules! impl_parse_arg_from_str {
	($($t:ty),* $(,)?) => {
		$(impl $crate::ParseArg for $t {
			fn parse_arg(s: &::std::ffi::OsStr) -> Result<Self, String> {
				let s = s.to_str()
					.ok_or_else(|| "value is not valid UTF-8".to_string())?;
				s.parse::<Self>().map_err(|e| e.to_string())
			}
		})*
	};
}

impl_parse_arg_from_str!(
	u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64, bool, char,
);

// ============================================================================
// Choice
// ============================================================================

/// A closed set of values, enumerated by `ALL` and rendered by `name`.
///
/// Implementing this gives you a `ParseArg` impl for free, and wires the
/// value list into "requires a value" errors and help output.
pub trait Choice: Sized + Copy + Display + 'static {
	/// Every variant, in the order they should appear in help.
	const ALL: &'static [Self];

	/// The CLI spelling of this variant.
	fn name(&self) -> &'static str;

	/// Look up a variant by its CLI spelling. The default is a linear scan
	/// over `ALL`, comparing `name()`. Override with a `match` for O(1)
	/// lookup on large enums.
	fn from_name(s: &str) -> Option<Self> {
		Self::ALL.iter().copied().find(|c| c.name() == s)
	}
}

impl<T: Choice> ParseArg for T {
	fn parse_arg(s: &OsStr) -> Result<Self, String> {
		let s = s.to_str().ok_or_else(|| "value is not valid UTF-8".to_string())?;
		Self::from_name(s).ok_or_else(|| format!("'{s}' is not a valid choice"))
	}

	fn values() -> Vec<&'static str> {
		Self::ALL.iter().map(|c| c.name()).collect()
	}
}

#[macro_export]
macro_rules! all_variants {
	($t:ident; $($v:ident),+ $(,)?) => {{
		// Exhaustive match: adding a variant to `$t` breaks compilation
		// here, forcing you to update `ALL`.
		#[allow(dead_code, clippy::match_single_binding)]
		const fn _exhaustive(c: $t) {
			match c {
				$($t::$v => {})+
			}
		}
		&[$($t::$v),+]
	}};
}

// ============================================================================
// TypeName
// ============================================================================

#[derive(Clone, Copy, Default)]
pub enum TypeName {
	#[default]
	Auto,
	Hidden,
	Manual(&'static str),
}

fn short_type_name<T>() -> &'static str {
	let full = std::any::type_name::<T>();
	full.rsplit_once("::").map_or(full, |(_, last)| last)
}

fn resolve_type_name<T>(tn: TypeName) -> Option<&'static str> {
	match tn {
		TypeName::Auto => Some(short_type_name::<T>()),
		TypeName::Hidden => None,
		TypeName::Manual(s) => Some(s),
	}
}

// ============================================================================
// kebab
// ============================================================================

/// Convert `snake_case` to `kebab-case`, borrowing for names without `_`.
///
/// `parse_args!` uses this to turn a struct field like `file_name` into the
/// CLI argument `--file-name`. Raw identifiers (`r#move`) lose the `r#`
/// prefix, so `r#move` becomes `--move`. Call it directly if you're using
/// the manual API and want the same convention.
pub fn kebab(s: &'static str) -> Cow<'static, str> {
	// `r#` is the raw-identifier prefix; strip it so `r#move` → `move`.
	let s = s.strip_prefix("r#").unwrap_or(s);
	if s.contains('_') {
		Cow::Owned(s.replace('_', "-"))
	} else {
		Cow::Borrowed(s)
	}
}

// ============================================================================
// Arg
// ============================================================================

#[derive(Clone, Copy)]
pub struct Arg {
	/// The short flag, or `'-'` for none. A lone `-` is treated as a
	/// positional by the parser, so it can never be a real short flag.
	pub short: char,
	/// Alternate long names. Accepted on the command line and shown in help.
	pub aliases: &'static [&'static str],
	pub desc: &'static str,
	pub allow_hyphen_values: bool,
	pub type_name: TypeName,
}

impl Default for Arg {
	fn default() -> Self {
		Arg {
			// `-` means "no short flag". A lone `-` is treated as a
			// positional by the parser, so it can never be a real short.
			short: '-',
			aliases: &[],
			desc: "",
			allow_hyphen_values: false,
			type_name: TypeName::Auto,
		}
	}
}

impl Arg {
	pub fn new() -> Self {
		Arg::default()
	}

	pub fn short(mut self, c: char) -> Self {
		self.short = c;
		self
	}

	pub fn aliases(mut self, names: &'static [&'static str]) -> Self {
		self.aliases = names;
		self
	}

	pub fn desc(mut self, d: &'static str) -> Self {
		self.desc = d;
		self
	}

	pub fn allow_hyphen_values(mut self) -> Self {
		self.allow_hyphen_values = true;
		self
	}

	/// Override the type name shown in error messages:
	/// `Arg::new().type_name("count")` prints `(expected count)` instead of
	/// `(expected u32)`.
	pub fn type_name(mut self, name: &'static str) -> Self {
		self.type_name = TypeName::Manual(name);
		self
	}

	/// Omit the type hint from error messages entirely.
	pub fn type_name_hidden(mut self) -> Self {
		self.type_name = TypeName::Hidden;
		self
	}
}

// ============================================================================
// Slot and typed keys
// ============================================================================

#[derive(Clone)]
enum Slot {
	Named(Cow<'static, str>),
	Positional {
		name: Cow<'static, str>,
		index: usize,
	},
}

impl Slot {
	fn label(&self) -> String {
		match self {
			Slot::Named(n) => format!("--{n}"),
			Slot::Positional { name, .. } => format!("<{name}>"),
		}
	}

	fn single<'a>(&self, args: &'a ParsedArgs<'a>) -> Option<&'a OsString> {
		match self {
			Slot::Named(n) => args.values.get(n.as_ref()).and_then(|v| v.first()),
			Slot::Positional { index, .. } => args.positionals.get(*index),
		}
	}

	fn all<'a>(&self, args: &'a ParsedArgs<'a>) -> &'a [OsString] {
		match self {
			Slot::Named(n) => args.values.get(n.as_ref()).map(Vec::as_slice).unwrap_or(&[]),
			Slot::Positional { index, .. } => {
				let start = (*index).min(args.positionals.len());
				&args.positionals[start..]
			}
		}
	}
}

pub struct RequiredKey<T> {
	slot: Slot,
	type_name: Option<&'static str>,
	_t: PhantomData<fn() -> T>,
}

pub struct OptionalKey<T> {
	slot: Slot,
	type_name: Option<&'static str>,
	_t: PhantomData<fn() -> T>,
}

pub struct DefaultKey<T> {
	slot: Slot,
	default: T,
	type_name: Option<&'static str>,
}

pub struct RepeatedKey<T> {
	slot: Slot,
	type_name: Option<&'static str>,
	_t: PhantomData<fn() -> T>,
}

#[derive(Clone)]
pub struct FlagKey {
	name: Cow<'static, str>,
}

impl<T> Clone for RequiredKey<T> {
	fn clone(&self) -> Self {
		RequiredKey { slot: self.slot.clone(), type_name: self.type_name, _t: PhantomData }
	}
}

impl<T> Clone for OptionalKey<T> {
	fn clone(&self) -> Self {
		OptionalKey { slot: self.slot.clone(), type_name: self.type_name, _t: PhantomData }
	}
}

impl<T: Clone> Clone for DefaultKey<T> {
	fn clone(&self) -> Self {
		DefaultKey {
			slot: self.slot.clone(),
			default: self.default.clone(),
			type_name: self.type_name,
		}
	}
}

impl<T> Clone for RepeatedKey<T> {
	fn clone(&self) -> Self {
		RepeatedKey { slot: self.slot.clone(), type_name: self.type_name, _t: PhantomData }
	}
}

// ============================================================================
// Parser internals
// ============================================================================

struct ArgDef {
	name: Cow<'static, str>,
	arg: Arg,
	takes_value: bool,
	required: bool,
	repeated: bool,
	values: Vec<&'static str>,
	default_display: Option<String>,
}

struct PositionalDef {
	name: Cow<'static, str>,
	desc: &'static str,
	required: bool,
	variadic: bool,
}

#[derive(Debug)]
pub enum ParseError {
	Help,
	Version,
	User(String),
}

impl Display for ParseError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ParseError::Help => write!(f, "help requested"),
			ParseError::Version => write!(f, "version requested"),
			ParseError::User(msg) => write!(f, "{msg}"),
		}
	}
}

impl std::error::Error for ParseError {}

pub struct ParsedArgs<'a> {
	parser: &'a ArgParser,
	values: HashMap<Cow<'static, str>, Vec<OsString>>,
	flags: HashSet<Cow<'static, str>>,
	positionals: Vec<OsString>,
}

// Hand-written so the output shows the parsed values, not the whole
// argument table. Deriving would also require `ArgParser: Debug`.
impl fmt::Debug for ParsedArgs<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ParsedArgs")
			.field("values", &self.values)
			.field("flags", &self.flags)
			.field("positionals", &self.positionals)
			.finish()
	}
}

// ============================================================================
// Classify
// ============================================================================

enum Classified<'a> {
	Separator,
	Positional,
	NonUtf8Token,
	/// The token is `--flag=<non-utf8>` or `-f=<non-utf8>`; the payload is
	/// the flag form, including the leading dashes.
	NonUtf8InlineValue(&'a str),
	Long {
		name: &'a str,
		inline: Option<&'a str>,
	},
	Shorts(&'a str),
}

fn classify(os: &OsStr) -> Classified<'_> {
	let bytes = os.as_encoded_bytes();
	if bytes == b"--" {
		return Classified::Separator;
	}
	let is_long = bytes.starts_with(b"--");
	let is_short = bytes.len() > 1 && bytes[0] == b'-';
	if !is_long && !is_short {
		return Classified::Positional;
	}
	match os.to_str() {
		None => {
			// `=` is ASCII, so a UTF-8 prefix before it stays UTF-8 even
			// when the value after it isn't. If the prefix decodes, we can
			// name the flag and suggest the separate-argument form.
			if let Some(eq) = bytes.iter().position(|&b| b == b'=') {
				if let Ok(flag) = std::str::from_utf8(&bytes[..eq]) {
					return Classified::NonUtf8InlineValue(flag);
				}
			}
			Classified::NonUtf8Token
		}
		Some(s) if is_long => match s[2..].split_once('=') {
			Some((name, inline)) => Classified::Long { name, inline: Some(inline) },
			None => Classified::Long { name: &s[2..], inline: None },
		},
		Some(s) => Classified::Shorts(&s[1..]),
	}
}

/// True if the token parses as an `i64` or `f64`, meaning it's a negative
/// number rather than a flag.
fn looks_like_number(s: &str) -> bool {
	s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok()
}

// ============================================================================
// ArgParser
// ============================================================================

#[derive(Default)]
struct State {
	values: HashMap<Cow<'static, str>, Vec<OsString>>,
	flags: HashSet<Cow<'static, str>>,
	positionals: Vec<OsString>,
	no_more_options: bool,
}

pub struct ArgParser {
	program: String,
	defs: Vec<ArgDef>,
	positionals: Vec<PositionalDef>,
	version: Option<&'static str>,
}

impl ArgParser {
	pub fn new(program: impl Into<String>) -> Self {
		ArgParser {
			program: program.into(),
			defs: Vec::new(),
			positionals: Vec::new(),
			version: None,
		}
	}

	pub fn set_version(&mut self, version: &'static str) {
		self.version = Some(version);
	}

	/// Print `msg` to stderr, then a `--help` hint, and exit with code 2.
	///
	/// Use this for validation failures after parsing has succeeded. Matches
	/// the shape of the parser's own type-error output.
	pub fn die(&self, msg: impl Display) -> ! {
		eprintln!("error: {msg}");
		eprintln!("Run with --help for usage.");
		std::process::exit(2);
	}

	// --- registration --------------------------------------------------

	pub fn required<T: ParseArg>(
		&mut self,
		name: impl Into<Cow<'static, str>>,
		arg: Arg,
	) -> RequiredKey<T> {
		let name = name.into();
		let tn = resolve_type_name::<T>(arg.type_name);
		self.push_named(name.clone(), arg, true, true, false, T::values(), None);
		RequiredKey { slot: Slot::Named(name), type_name: tn, _t: PhantomData }
	}

	pub fn default<T: ParseArg + Display>(
		&mut self,
		name: impl Into<Cow<'static, str>>,
		default: T,
		arg: Arg,
	) -> DefaultKey<T> {
		let name = name.into();
		let tn = resolve_type_name::<T>(arg.type_name);
		self.push_named(
			name.clone(),
			arg,
			true,
			false,
			false,
			T::values(),
			Some(default.to_string()),
		);
		DefaultKey { slot: Slot::Named(name), default, type_name: tn }
	}

	pub fn optional<T: ParseArg>(
		&mut self,
		name: impl Into<Cow<'static, str>>,
		arg: Arg,
	) -> OptionalKey<T> {
		let name = name.into();
		let tn = resolve_type_name::<T>(arg.type_name);
		self.push_named(name.clone(), arg, true, false, false, T::values(), None);
		OptionalKey { slot: Slot::Named(name), type_name: tn, _t: PhantomData }
	}

	pub fn repeated<T: ParseArg>(
		&mut self,
		name: impl Into<Cow<'static, str>>,
		arg: Arg,
	) -> RepeatedKey<T> {
		let name = name.into();
		let tn = resolve_type_name::<T>(arg.type_name);
		self.push_named(name.clone(), arg, true, false, true, T::values(), None);
		RepeatedKey { slot: Slot::Named(name), type_name: tn, _t: PhantomData }
	}

	pub fn flag(&mut self, name: impl Into<Cow<'static, str>>, arg: Arg) -> FlagKey {
		let name = name.into();
		self.push_named(name.clone(), arg, false, false, false, Vec::new(), None);
		FlagKey { name }
	}

	pub fn positional<T: ParseArg>(
		&mut self,
		name: impl Into<Cow<'static, str>>,
		arg: Arg,
	) -> RequiredKey<T> {
		let name = name.into();
		self.push_positional(name.clone(), &arg, true, false);
		RequiredKey {
			slot: Slot::Positional { name, index: self.positionals.len() - 1 },
			type_name: resolve_type_name::<T>(arg.type_name),
			_t: PhantomData,
		}
	}

	pub fn positional_optional<T: ParseArg>(
		&mut self,
		name: impl Into<Cow<'static, str>>,
		arg: Arg,
	) -> OptionalKey<T> {
		let name = name.into();
		self.push_positional(name.clone(), &arg, false, false);
		OptionalKey {
			slot: Slot::Positional { name, index: self.positionals.len() - 1 },
			type_name: resolve_type_name::<T>(arg.type_name),
			_t: PhantomData,
		}
	}

	pub fn positional_variadic<T: ParseArg>(
		&mut self,
		name: impl Into<Cow<'static, str>>,
		arg: Arg,
	) -> RepeatedKey<T> {
		let name = name.into();
		assert!(
			self.positionals.iter().all(|p| !p.variadic),
			"only one variadic positional allowed, and it must be last"
		);
		self.push_positional(name.clone(), &arg, false, true);
		RepeatedKey {
			slot: Slot::Positional { name, index: self.positionals.len() - 1 },
			type_name: resolve_type_name::<T>(arg.type_name),
			_t: PhantomData,
		}
	}

	fn push_positional(
		&mut self,
		name: Cow<'static, str>,
		arg: &Arg,
		required: bool,
		variadic: bool,
	) {
		assert!(arg.short == '-', "positional {name} must not have a short");
		assert!(!name.is_empty(), "argument name must not be empty");
		assert!(
			self.positionals.iter().all(|p| !p.variadic),
			"cannot declare a positional after a variadic positional"
		);
		if !variadic {
			assert!(
				self.positionals.iter().all(|p| p.required || !required),
				"required positionals must be declared before optional ones"
			);
		}
		self.positionals
			.push(PositionalDef { name, desc: arg.desc, required, variadic });
	}

	#[allow(clippy::too_many_arguments)]
	fn push_named(
		&mut self,
		name: Cow<'static, str>,
		arg: Arg,
		takes_value: bool,
		required: bool,
		repeated: bool,
		values: Vec<&'static str>,
		default_display: Option<String>,
	) {
		assert!(!name.is_empty(), "argument name must not be empty");
		assert!(!self.defs.iter().any(|d| d.name == name), "duplicate argument name: {name}");
		if arg.short != '-' {
			assert!(
				!self.defs.iter().any(|d| d.arg.short == arg.short),
				"duplicate short option: -{}",
				arg.short
			);
		}
		for alias in arg.aliases {
			assert!(
				!self
					.defs
					.iter()
					.any(|d| { d.name.as_ref() == *alias || d.arg.aliases.contains(alias) }),
				"alias '{alias}' collides with an existing argument"
			);
		}
		self.defs.push(ArgDef {
			name,
			arg,
			takes_value,
			required,
			repeated,
			values,
			default_display,
		});
	}

	// --- parsing -------------------------------------------------------

	pub fn parse<I>(&self, raw: I) -> ParsedArgs<'_>
	where
		I: IntoIterator,
		I::Item: Into<OsString>,
	{
		match self.try_parse(raw) {
			Ok(p) => p,
			Err(ParseError::Help) => {
				print!("{}", self.help());
				std::process::exit(0);
			}
			Err(ParseError::Version) => {
				if let Some(v) = self.version {
					println!("{v}");
				}
				std::process::exit(0);
			}
			Err(ParseError::User(msg)) => {
				eprintln!("error: {msg}\n");
				eprint!("{}", self.help());
				std::process::exit(2);
			}
		}
	}

	pub fn try_parse<I>(&self, raw: I) -> Result<ParsedArgs<'_>, ParseError>
	where
		I: IntoIterator,
		I::Item: Into<OsString>,
	{
		let mut it = raw.into_iter().map(Into::into).peekable();
		let _argv0 = it.next();
		let mut state = State::default();

		while let Some(os) = it.next() {
			if state.no_more_options {
				state.positionals.push(os);
				continue;
			}
			match classify(&os) {
				Classified::Separator => state.no_more_options = true,
				Classified::Positional => state.positionals.push(os),
				Classified::NonUtf8Token => {
					return Err(ParseError::User(format!(
						"non-UTF-8 option: {}",
						os.to_string_lossy()
					)));
				}
				Classified::NonUtf8InlineValue(flag) => {
					return Err(ParseError::User(format!(
						"non-UTF-8 value in {flag}=…; use `{flag} VALUE` instead"
					)));
				}
				Classified::Long { name, inline } => {
					self.parse_long(&mut it, name, inline, &mut state)?;
				}
				Classified::Shorts(rest) => {
					self.parse_short(&mut it, rest, &mut state)?;
				}
			}
		}

		self.check_required(&state)?;
		self.check_positionals(&state.positionals)?;

		Ok(ParsedArgs {
			parser: self,
			values: state.values,
			flags: state.flags,
			positionals: state.positionals,
		})
	}

	fn parse_long<I: Iterator<Item = OsString>>(
		&self,
		it: &mut std::iter::Peekable<I>,
		name: &str,
		inline: Option<&str>,
		state: &mut State,
	) -> Result<(), ParseError> {
		if let Some(e) = self.builtin(name, None) {
			return Err(e);
		}
		let def = self
			.find_long(name)
			.ok_or_else(|| ParseError::User(format!("unknown option --{name}")))?;
		self.apply(def, inline.map(OsString::from), it, None, state)
	}

	fn parse_short<I: Iterator<Item = OsString>>(
		&self,
		it: &mut std::iter::Peekable<I>,
		rest: &str,
		state: &mut State,
	) -> Result<(), ParseError> {
		let chars: Vec<char> = rest.chars().collect();
		let mut i = 0;
		while i < chars.len() {
			let c = chars[i];
			if let Some(e) = self.builtin("", Some(c)) {
				return Err(e);
			}
			let def = self
				.find_short(c)
				.ok_or_else(|| ParseError::User(format!("unknown option -{c}")))?;
			if def.takes_value {
				let tail: String = chars[i + 1..].iter().collect();
				let inline = if tail.is_empty() {
					None
				} else {
					let v = tail.strip_prefix('=').unwrap_or(&tail);
					Some(OsString::from(v))
				};
				return self.apply(def, inline, it, Some(c), state);
			}
			state.flags.insert(def.name.clone());
			i += 1;
			// A flag followed by `=` is the `-v=yes` form, which is an error.
			// Without this check, the `=` would be reported as `unknown
			// option -=`.
			if i < chars.len() && chars[i] == '=' {
				return Err(ParseError::User(format!("--{} does not take a value", def.name)));
			}
		}
		Ok(())
	}

	/// Returns `Some(Help)` or `Some(Version)` if the token is a built-in and
	/// the user hasn't registered their own under that name.
	fn builtin(&self, name: &str, short: Option<char>) -> Option<ParseError> {
		if short == Some('h') && self.find_short('h').is_none() {
			return Some(ParseError::Help);
		}
		if short == Some('V') && self.find_short('V').is_none() && self.version.is_some() {
			return Some(ParseError::Version);
		}
		if name == "help" && self.find_long("help").is_none() {
			return Some(ParseError::Help);
		}
		if name == "version" && self.find_long("version").is_none() && self.version.is_some() {
			return Some(ParseError::Version);
		}
		None
	}

	/// The single place that turns a matched definition into parser state.
	/// Consumes the next token as a value if one wasn't given inline.
	fn apply<I: Iterator<Item = OsString>>(
		&self,
		def: &ArgDef,
		inline: Option<OsString>,
		it: &mut std::iter::Peekable<I>,
		short: Option<char>,
		state: &mut State,
	) -> Result<(), ParseError> {
		if def.takes_value {
			let value = match inline {
				Some(v) => v,
				None => self.consume_value(it, def, short)?,
			};
			let entry = state.values.entry(def.name.clone()).or_default();
			// A non-repeated option given twice is almost always a mistake,
			// so it's an error rather than "last wins".
			if !def.repeated && !entry.is_empty() {
				return Err(ParseError::User(format!("--{} given more than once", def.name)));
			}
			entry.push(value);
			Ok(())
		} else {
			if inline.is_some() {
				return Err(ParseError::User(format!("--{} does not take a value", def.name)));
			}
			state.flags.insert(def.name.clone());
			Ok(())
		}
	}

	fn consume_value<I: Iterator<Item = OsString>>(
		&self,
		it: &mut std::iter::Peekable<I>,
		def: &ArgDef,
		short: Option<char>,
	) -> Result<OsString, ParseError> {
		let flag_form = match short {
			Some(c) => format!("-{c}"),
			None => format!("--{}", def.name),
		};
		let next = it.next().ok_or_else(|| {
			let hint = if def.values.is_empty() {
				String::new()
			} else {
				format!(" (one of: {})", def.values.join(", "))
			};
			ParseError::User(format!("{flag_form} requires a value{hint}"))
		})?;
		if next == "--" {
			return Err(ParseError::User(format!("{flag_form} requires a value, found `--`")));
		}
		let bytes = next.as_encoded_bytes();
		// `--offset -5` is common enough that requiring `allow_hyphen_values`
		// for it would be friction. Any token that parses as a number is
		// accepted; everything else starting with `-` still needs the flag.
		if !def.arg.allow_hyphen_values
			&& bytes.len() > 1
			&& bytes[0] == b'-'
			&& next != "-"
			&& !next.to_str().is_some_and(looks_like_number)
		{
			return Err(ParseError::User(format!(
				"{flag_form}: value '{}' looks like a flag; \
				 use {flag_form}=VALUE or `--` to separate",
				next.to_string_lossy()
			)));
		}
		Ok(next)
	}

	fn check_required(&self, state: &State) -> Result<(), ParseError> {
		for def in &self.defs {
			if def.required && !state.values.contains_key(&def.name) {
				return Err(ParseError::User(format!("missing required argument --{}", def.name)));
			}
		}
		Ok(())
	}

	fn check_positionals(&self, provided: &[OsString]) -> Result<(), ParseError> {
		let has_variadic = self.positionals.iter().any(|p| p.variadic);
		// The variadic slot, if present, is always last and takes the rest,
		// so it's excluded from the "fixed slots" count.
		let fixed = self.positionals.len() - has_variadic as usize;

		for (i, def) in self.positionals.iter().take(fixed).enumerate() {
			if def.required && i >= provided.len() {
				return Err(ParseError::User(format!(
					"missing required positional argument: {}",
					def.name
				)));
			}
		}
		if !has_variadic && provided.len() > fixed {
			return Err(ParseError::User(format!(
				"unexpected positional argument: {}",
				provided[fixed].to_string_lossy()
			)));
		}
		Ok(())
	}

	fn find_long(&self, name: &str) -> Option<&ArgDef> {
		self.defs
			.iter()
			.find(|d| d.name.as_ref() == name || d.arg.aliases.contains(&name))
	}

	fn find_short(&self, c: char) -> Option<&ArgDef> {
		// `-` is the sentinel for "no short", not a matchable short.
		if c == '-' {
			return None;
		}
		self.defs.iter().find(|d| d.arg.short == c)
	}

	// --- help ----------------------------------------------------------

	pub fn help(&self) -> String {
		let mut out = format!("Usage: {}", self.program);
		if !self.defs.is_empty() || self.version.is_some() {
			out.push_str(" [OPTIONS]");
		}
		for p in &self.positionals {
			if p.variadic {
				out.push_str(&format!(" [{}...]", p.name));
			} else if p.required {
				out.push_str(&format!(" <{}>", p.name));
			} else {
				out.push_str(&format!(" [{}]", p.name));
			}
		}
		out.push('\n');

		let pos_rows = self.positional_rows();
		let opt_rows = self.option_rows();
		let w = pos_rows.iter().chain(&opt_rows).map(|(l, _)| l.len()).max().unwrap_or(0);

		if !pos_rows.is_empty() {
			out.push_str("\nArguments:\n");
			push_rows(&mut out, &pos_rows, w);
		}
		out.push_str("\nOptions:\n");
		push_rows(&mut out, &opt_rows, w);

		out
	}

	fn positional_rows(&self) -> Vec<(String, String)> {
		self.positionals
			.iter()
			.map(|p| {
				let left = if p.variadic {
					format!("<{}>...", p.name)
				} else {
					format!("<{}>", p.name)
				};
				let mut right = p.desc.to_string();
				if p.required {
					push_tag(&mut right, "[required]");
				}
				(left, right)
			})
			.collect()
	}

	fn option_rows(&self) -> Vec<(String, String)> {
		let mut rows: Vec<(String, String)> = self
			.defs
			.iter()
			.map(|d| {
				let mut left = if d.arg.short == '-' {
					format!("    --{}", d.name)
				} else {
					format!("-{}, --{}", d.arg.short, d.name)
				};
				for alias in d.arg.aliases {
					left.push_str(&format!(", --{alias}"));
				}
				if d.takes_value {
					left.push_str(&format!(" <{}>", d.name.to_uppercase()));
				}
				let mut right = d.arg.desc.to_string();
				if !d.values.is_empty() {
					push_tag(&mut right, &format!("[{}]", d.values.join("|")));
				}
				if d.required {
					push_tag(&mut right, "[required]");
				} else if let Some(def) = &d.default_display {
					push_tag(&mut right, &format!("[default: {def}]"));
				}
				(left, right)
			})
			.collect();
		rows.push(("-h, --help".into(), "Print help".into()));
		if self.version.is_some() {
			rows.push(("-V, --version".into(), "Print version".into()));
		}
		rows
	}

	fn values_for(&self, slot: &Slot) -> &[&'static str] {
		let Slot::Named(name) = slot else { return &[] };
		self.defs
			.iter()
			.find(|d| d.name == *name)
			.map(|d| d.values.as_slice())
			.unwrap_or(&[])
	}
}

fn push_tag(right: &mut String, tag: &str) {
	if !right.is_empty() {
		right.push(' ');
	}
	right.push_str(tag);
}

fn push_rows(out: &mut String, rows: &[(String, String)], w: usize) {
	for (l, r) in rows {
		out.push_str(&format!("  {:<w$}  {}\n", l, r, w = w));
	}
}

// ============================================================================
// Key accessors
// ============================================================================

fn parse_or_exit<T: ParseArg>(
	parser: &ArgParser,
	raw: &OsStr,
	slot: &Slot,
	type_name: Option<&'static str>,
) -> T {
	match T::parse_arg(raw) {
		Ok(v) => v,
		Err(e) => {
			let label = slot.label();
			let values = parser.values_for(slot);
			let (hint, detail) = if values.is_empty() {
				let hint = type_name.map(|t| format!(" (expected {t})")).unwrap_or_default();
				(hint, format!(": {e}"))
			} else {
				(format!(" (one of: {})", values.join(", ")), String::new())
			};
			parser.die(format!(
				"invalid value '{}' for {label}{hint}{detail}",
				raw.to_string_lossy()
			));
		}
	}
}

impl<T: ParseArg> RequiredKey<T> {
	pub fn get(&self, args: &ParsedArgs<'_>) -> T {
		let raw = self
			.slot
			.single(args)
			.unwrap_or_else(|| panic!("programmer error: {} not provided", self.slot.label()));
		parse_or_exit(args.parser, raw, &self.slot, self.type_name)
	}
}

impl<T: ParseArg + Clone> DefaultKey<T> {
	pub fn get(&self, args: &ParsedArgs<'_>) -> T {
		match self.slot.single(args) {
			Some(raw) => parse_or_exit(args.parser, raw, &self.slot, self.type_name),
			None => self.default.clone(),
		}
	}
}

impl<T: ParseArg> OptionalKey<T> {
	pub fn get(&self, args: &ParsedArgs<'_>) -> Option<T> {
		self.slot
			.single(args)
			.map(|raw| parse_or_exit(args.parser, raw, &self.slot, self.type_name))
	}
}

impl<T: ParseArg> RepeatedKey<T> {
	pub fn get(&self, args: &ParsedArgs<'_>) -> Vec<T> {
		self.slot
			.all(args)
			.iter()
			.map(|raw| parse_or_exit(args.parser, raw, &self.slot, self.type_name))
			.collect()
	}

	pub fn len(&self, args: &ParsedArgs<'_>) -> usize {
		self.slot.all(args).len()
	}

	pub fn is_empty(&self, args: &ParsedArgs<'_>) -> bool {
		self.slot.all(args).is_empty()
	}
}

impl FlagKey {
	pub fn get(&self, args: &ParsedArgs<'_>) -> bool {
		args.flags.contains(&self.name)
	}
}

// ============================================================================
// parse_args!
// ============================================================================

/// Parse arguments and build an instance of `$T`.
///
/// ```ignore
/// let args = parse_args!(parser; Args {
///     host: required(Arg::new().short('H').desc("Host")),
///     verbose: flag(Arg::new().short('v')),
/// });
/// ```
///
/// The struct field name becomes the CLI argument name, converted to
/// kebab-case: `file_name` becomes `--file-name`. Raw identifiers lose the
/// `r#` prefix: `r#move` becomes `--move`. The struct field type fixes the
/// parser's type parameter, so no turbofish is needed. Each argument is
/// declared once.
///
/// For tests, use the explicit-argv form:
///
/// ```ignore
/// let args = parse_args!(parser, argv; Args { ... });
/// ```
///
/// `$parser` is a parameter because `macro_rules!` hygiene prevents the macro
/// body from referring to a caller binding by name.
#[macro_export]
macro_rules! parse_args {
	($parser:ident; $T:ident { $($field:ident : $method:ident ( $($tail:tt)* )),* $(,)? }) => {
		$crate::parse_args!($parser, ::std::env::args_os(); $T {
			$($field: $method ( $($tail)* )),*
		})
	};
	($parser:ident, $argv:expr; $T:ident { $($field:ident : $method:ident ( $($tail:tt)* )),* $(,)? }) => {{
		$(
			// The struct field name becomes the CLI name, converted to
			// kebab-case: `file_name` → `--file-name`.
			let $field = $parser.$method(
				$crate::kebab(stringify!($field)),
				$($tail)*
			);
		)*
		let args = $parser.parse($argv);
		$T { $($field: $field.get(&args),)* }
	}};
}

// ============================================================================
// Subcommand dispatch
// ============================================================================

/// A subcommand: a name, a one-line description, and a handler.
///
/// The handler receives the program name (for `--help` output) and the
/// remaining argv with the command name still in position 0 — the same shape
/// a fresh `ArgParser` expects, so the handler can call `parser.parse(argv)`
/// directly and the parser skips the command name as `argv[0]`.
#[derive(Clone, Copy)]
pub struct Command {
	pub name: &'static str,
	pub handler: fn(prog: &str, argv: &[OsString]),
	pub desc: &'static str,
}

pub struct Dispatcher {
	program: String,
	commands: Vec<Command>,
	help_cmd: Option<(&'static str, &'static str)>,
}

impl Dispatcher {
	/// `program` is used for `help()` called directly (tests, banners). At
	/// runtime, `argv[0]` takes precedence so the printed name matches how
	/// the binary was invoked.
	pub fn new(program: impl Into<String>) -> Self {
		Dispatcher { program: program.into(), commands: Vec::new(), help_cmd: None }
	}

	pub fn command(
		&mut self,
		name: &'static str,
		handler: fn(&str, &[OsString]),
		desc: &'static str,
	) -> &mut Self {
		self.commands.push(Command { name, handler, desc });
		self
	}

	/// Register `name` as a help command. `prog name` prints the
	/// dispatcher's help and exits 0, same as `--help`. The command is
	/// listed in the help output. `-h`, `--help`, and `--` work whether
	/// or not this is called.
	pub fn help_command(&mut self, name: &'static str, desc: &'static str) -> &mut Self {
		self.help_cmd = Some((name, desc));
		self
	}

	/// Run `argv[1]` as a subcommand name.
	///
	/// - no argument, `-h`, `--help`, `--`, or the registered help command
	///   → prints help, exits 0
	/// - known command → calls its handler, returns normally
	/// - unknown → prints error and help, exits 2
	pub fn run<I>(&self, argv: I)
	where
		I: IntoIterator,
		I::Item: Into<OsString>,
	{
		let argv: Vec<OsString> = argv.into_iter().map(Into::into).collect();
		let prog = self.program_from(&argv);

		let name = argv.get(1).and_then(|s| s.to_str());
		let is_help = name.is_none()
			|| matches!(name, Some("-h") | Some("--help") | Some("--"))
			|| self.help_cmd.is_some_and(|(hn, _)| name == Some(hn));

		if is_help {
			print!("{}", self.help_with(&prog));
			std::process::exit(0);
		}

		let name = name.expect("handled above");
		match self.find(name) {
			Some(cmd) => (cmd.handler)(&prog, &argv[1..]),
			None => {
				eprintln!("error: unknown command '{name}'");
				eprint!("{}", self.help_with(&prog));
				std::process::exit(2);
			}
		}
	}

	pub fn find(&self, name: &str) -> Option<&Command> {
		self.commands.iter().find(|c| c.name == name)
	}

	pub fn help(&self) -> String {
		self.help_with(&self.program)
	}

	fn program_from(&self, argv: &[OsString]) -> String {
		argv.first()
			.map(|p| p.to_string_lossy().into_owned())
			.unwrap_or_else(|| self.program.clone())
	}

	fn help_with(&self, prog: &str) -> String {
		let mut rows: Vec<(&str, &str)> = self.commands.iter().map(|c| (c.name, c.desc)).collect();
		if let Some((name, desc)) = self.help_cmd {
			rows.push((name, desc));
		}
		rows.push(("-h, --help", "Print help"));

		let w = rows.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
		let mut out = format!("Usage: {prog} <command> [args]\n\nCommands:\n");
		for (name, desc) in rows {
			out.push_str(&format!("  {:<w$}  {}\n", name, desc, w = w));
		}
		out
	}
}
