use std::{ffi::OsString, path::PathBuf};

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

fn cmd_add(prog: &str, argv: &[OsString]) -> Result<Outcome, ParseError> {
	let mut parser = ArgParser::new(format!("{prog} add"));
	let args = AddArgs::try_parse_from(&mut parser, argv)?;
	if args.files.is_empty() {
		return Err(ParseError::User("at least one file is required".into()));
	}
	Ok(Outcome::Added { collection: args.collection_name, files: args.files })
}

fn cmd_remove(prog: &str, argv: &[OsString]) -> Result<Outcome, ParseError> {
	let mut parser = ArgParser::new(format!("{prog} remove"));
	let args = RemoveArgs::try_parse_from(&mut parser, argv)?;
	if !args.force {
		return Err(ParseError::User("refusing to remove without --force".into()));
	}
	Ok(Outcome::Removed { collection: args.collection_name })
}

fn app() -> Dispatcher<Result<Outcome, ParseError>> {
	let mut dispatcher = Dispatcher::new("myprog");
	dispatcher.command("add", cmd_add, "Add files to a collection");
	dispatcher.command("remove", cmd_remove, "Remove a collection");
	dispatcher.help_command("help", "Print help");
	dispatcher
}

fn main() {
	println!("{:#?}", app().run_with(std::env::args_os()));
}

// Run with: cargo test --example subcommands_value
#[cfg(test)]
mod tests {
	use super::*;
	use miniargs::Dispatch;

	fn argv(parts: &[&str]) -> Vec<OsString> {
		parts.iter().map(OsString::from).collect()
	}

	#[test]
	fn dispatch_returns_values() {
		let dispatcher = app();
		assert!(matches!(
			dispatcher.dispatch_with(argv(&["myprog", "add", "-n", "docs", "a.txt"])),
			Dispatch::Ran(Ok(Outcome::Added { .. }))
		));
		assert!(matches!(dispatcher.dispatch_with(argv(&["myprog", "--help"])), Dispatch::Help));
	}
}
