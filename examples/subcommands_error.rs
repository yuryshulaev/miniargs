use std::{ffi::OsString, path::PathBuf};

use miniargs::{arg, args, ArgParser, Dispatcher, FatalError, ParseError};

args! {
	#[derive(Debug)]
	struct AddArgs {
		collection_name: String = required(arg().short('n').desc("Collection to add to")),
		force: bool = flag(arg().short('f').desc("Overwrite existing entries")),
		files: Vec<PathBuf> = positional_variadic(arg().desc("Files to add")),
	}
}

args! {
	#[derive(Debug)]
	struct RemoveArgs {
		collection_name: String = required(arg().short('n').desc("Collection to remove")),
		force: bool = flag(arg().short('f').desc("Remove without confirmation")),
	}
}

#[derive(Debug, PartialEq)]
enum Outcome {
	Added {
		collection: String,
		files: Vec<PathBuf>,
	},
	Removed {
		collection: String,
	},
}

#[derive(Debug)]
enum AppError {
	Parse(ParseError),
	Domain(String),
}

impl From<ParseError> for AppError {
	fn from(e: ParseError) -> Self {
		AppError::Parse(e)
	}
}

impl FatalError for AppError {
	fn exit(self) -> ! {
		match self {
			AppError::Parse(e) => e.exit(),
			AppError::Domain(msg) => ParseError::User(msg).exit(),
		}
	}
}

fn cmd_add(prog: &str, argv: &[OsString]) -> Result<Outcome, AppError> {
	let mut parser = ArgParser::new(format!("{prog} add"));
	let args = AddArgs::try_parse_from(&mut parser, argv)?;

	if args.files.is_empty() {
		return Err(AppError::Domain("at least one file is required".into()));
	}
	if !args.force && args.files.len() > 3 {
		return Err(AppError::Domain(format!(
			"refusing to add {} files without --force",
			args.files.len()
		)));
	}

	Ok(Outcome::Added { collection: args.collection_name, files: args.files })
}

fn cmd_remove(prog: &str, argv: &[OsString]) -> Result<Outcome, AppError> {
	let mut parser = ArgParser::new(format!("{prog} remove"));
	let args = RemoveArgs::try_parse_from(&mut parser, argv)?;

	if !args.force {
		return Err(AppError::Domain("refusing to remove without --force".into()));
	}

	Ok(Outcome::Removed { collection: args.collection_name })
}

fn app() -> Dispatcher<Result<Outcome, AppError>> {
	let mut dispatcher = Dispatcher::new("myprog");
	dispatcher.set_version(env!("CARGO_PKG_VERSION"));
	dispatcher.command("add", cmd_add, "Add files to a collection");
	dispatcher.command("remove", cmd_remove, "Remove a collection");
	dispatcher.help_command("help", "Print help");
	dispatcher
}

fn main() {
	match app().run() {
		Outcome::Added { collection, files } => {
			for f in &files {
				println!("Added {f:?} to {collection}");
			}
		}
		Outcome::Removed { collection } => println!("Removed {collection}"),
	}
}

// Run with: cargo test --example subcommands_error
#[cfg(test)]
mod tests {
	use super::*;
	use miniargs::Dispatch;

	fn argv(parts: &[&str]) -> Vec<OsString> {
		parts.iter().map(OsString::from).collect()
	}

	#[test]
	fn dispatch_returns_values() {
		let d = app();
		assert!(matches!(
			d.dispatch_with(argv(&["myprog", "add", "-n", "docs", "a.txt"])),
			Dispatch::Ran(Ok(Outcome::Added { .. }))
		));
		assert!(matches!(d.dispatch_with(argv(&["myprog", "--help"])), Dispatch::Help));
	}

	#[test]
	fn domain_errors_are_distinguishable() {
		let d = app();
		assert!(matches!(
			d.dispatch_with(argv(&["myprog", "add", "-n", "docs"])),
			Dispatch::Ran(Err(AppError::Domain(_)))
		));
		assert!(matches!(
			d.dispatch_with(argv(&["myprog", "add", "-n", "docs", "a", "b", "c", "d"])),
			Dispatch::Ran(Err(AppError::Domain(_)))
		));
	}

	#[test]
	fn parse_errors_are_distinguishable() {
		let d = app();
		assert!(matches!(
			d.dispatch_with(argv(&["myprog", "add", "a.txt"])),
			Dispatch::Ran(Err(AppError::Parse(_)))
		));
	}

	#[test]
	fn subcommand_help_is_a_value() {
		let d = app();
		// `myprog add --help` propagates `ParseError::Help` through the
		// handler's `?` and lands in `AppError::Parse`. The dispatcher
		// would call `AppError::exit`, which delegates to `ParseError::exit`
		// to print the subcommand's help and exit 0.
		let out = d.dispatch_with(argv(&["myprog", "add", "--help"]));
		let Dispatch::Ran(Err(AppError::Parse(ParseError::Help(text)))) = out else {
			panic!("expected nested Help");
		};
		assert!(text.contains("--collection-name"), "{text}");
	}

	#[test]
	fn terminal_errors_survive_conversion() {
		let e = AppError::from(ParseError::Help("help text".into()));
		assert!(matches!(
			e,
			AppError::Parse(ParseError::Help(ref t)) if t == "help text"
		));

		let e = AppError::from(ParseError::Version("1.0".into()));
		assert!(matches!(
			e,
			AppError::Parse(ParseError::Version(ref v)) if v == "1.0"
		));
	}
}
