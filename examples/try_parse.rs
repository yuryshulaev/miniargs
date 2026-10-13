use std::{ffi::OsString, path::PathBuf};

use miniargs::{arg, args, ArgParser, FatalError, ParseError};

args! {
	#[derive(Debug)]
	struct Args {
		target: String = required(arg().short('t').desc("Build target")),
		verbose: bool = flag(arg().short('v').desc("Verbose output")),
		files: Vec<PathBuf> = positional_variadic(arg().desc("Input files")),
	}
}

#[derive(Debug, PartialEq)]
struct Config {
	target: String,
	verbose: bool,
	files: Vec<PathBuf>,
}

#[derive(Debug)]
enum AppError {
	Parse(ParseError),
	Domain(String),
}

impl FatalError for AppError {
	fn exit(self) -> ! {
		match self {
			AppError::Parse(e) => e.exit(),
			AppError::Domain(msg) => ParseError::User(msg).exit(),
		}
	}
}

fn main() {
	let mut parser = ArgParser::new("mytool");
	parser.set_version(env!("CARGO_PKG_VERSION"));
	let argv: Vec<OsString> = std::env::args_os().collect();
	let config = config(&mut parser, &argv).unwrap_or_else(|e| e.exit());
	run(config);
}

/// Parse and validate. Every outcome (including `--help` and `--version`)
/// comes back as a value. No printing, no exiting.
fn config(parser: &mut ArgParser, argv: &[OsString]) -> Result<Config, AppError> {
	let args = Args::try_parse_from(parser, argv).map_err(AppError::Parse)?;

	if args.files.is_empty() {
		return Err(AppError::Domain("at least one file is required".into()));
	}

	Ok(Config { target: args.target, verbose: args.verbose, files: args.files })
}

fn run(config: Config) {
	for file in &config.files {
		println!("Building {file:?} for {}", config.target);
	}
	if config.verbose {
		println!("{} file(s) total", config.files.len());
	}
}

// Run with: cargo test --example try_parse
#[cfg(test)]
mod tests {
	use super::*;

	fn argv(parts: &[&str]) -> Vec<OsString> {
		parts.iter().map(OsString::from).collect()
	}

	fn parse(parts: &[&str]) -> Result<Config, AppError> {
		config(&mut ArgParser::new("mytool"), &argv(parts))
	}

	#[test]
	fn parses_to_config() {
		let Ok(c) = parse(&["mytool", "-t", "release", "-v", "a.txt"]) else {
			panic!("expected Ok");
		};
		assert_eq!(c.target, "release");
		assert!(c.verbose);
		assert_eq!(c.files, vec![PathBuf::from("a.txt")]);
	}

	#[test]
	fn help_is_a_value() {
		let Err(AppError::Parse(ParseError::Help(text))) = parse(&["mytool", "--help"]) else {
			panic!("expected Help");
		};
		assert!(text.contains("--target"), "{text}");
	}

	#[test]
	fn rejects_empty_files() {
		let Err(AppError::Domain(msg)) = parse(&["mytool", "-t", "release"]) else {
			panic!("expected Domain error");
		};
		assert!(msg.contains("at least one file"), "{msg}");
	}
}
