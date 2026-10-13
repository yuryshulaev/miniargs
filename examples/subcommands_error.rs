use std::{ffi::OsString, fmt, path::PathBuf};

use miniargs::{arg, args, ArgParser, Dispatcher, ParseError};

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

/// What each subcommand produces. This is what tests assert on.
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

/// Everything a handler can fail with: argument errors from the parser,
/// plus whatever the command's own rules reject.
#[derive(Debug)]
enum AppError {
	Parse(ParseError),
	/// The command's own validation, separate from argument parsing.
	Domain(String),
}

impl From<ParseError> for AppError {
	fn from(e: ParseError) -> Self {
		AppError::Parse(e)
	}
}

impl fmt::Display for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			AppError::Parse(e) => write!(f, "{e}"),
			AppError::Domain(msg) => write!(f, "{msg}"),
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
	match app().run_with(std::env::args_os()) {
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
		// Missing required --collection-name reaches AppError::Parse,
		// because try_parse_from propagates only ParseError::User.
		assert!(matches!(
			d.dispatch_with(argv(&["myprog", "add", "a.txt"])),
			Dispatch::Ran(Err(AppError::Parse(_)))
		));
	}
}
