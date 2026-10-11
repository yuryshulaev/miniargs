use std::path::PathBuf;

use miniargs::{Arg, ArgParser};

fn main() {
	let mut parser = ArgParser::new("pack");
	parser.set_version(env!("CARGO_PKG_VERSION"));

	// Required named option.
	let target = parser.required("target", Arg::new().short('t').desc("Build target"));

	// Defaulted named option.
	let jobs = parser.default(
		"jobs",
		1u32,
		Arg::new().short('j').desc("Parallel jobs").type_name("count"),
	);

	// Optional named option.
	let project_name = parser.optional(
		"project-name", //
		Arg::new().desc("Project name (defaults to directory name)"),
	);

	// Optional named option with a hidden type name.
	let log = parser.optional("log", Arg::new().desc("Log file path").type_name_hidden());

	// Flag.
	let verbose = parser.flag("verbose", Arg::new().short('v').desc("Verbose output"));

	// Repeated named option.
	let include = parser.repeated("include", Arg::new().short('I').desc("Add include directory"));

	// Positional (required).
	let output = parser.positional("output", Arg::new().desc("Output file"));

	// Positional (variadic): zero or more, must be last.
	let inputs = parser.positional_variadic("inputs", Arg::new().desc("Input files"));

	let args = parser.parse(std::env::args_os());

	let target: String = target.get(&args);
	let jobs: u32 = jobs.get(&args);
	let project_name: Option<String> = project_name.get(&args);
	let log: Option<PathBuf> = log.get(&args);
	let verbose: bool = verbose.get(&args);
	let include: Vec<PathBuf> = include.get(&args);
	let output: PathBuf = output.get(&args);
	let inputs: Vec<PathBuf> = inputs.get(&args);

	if inputs.is_empty() {
		parser.die("at least one input file is required");
	}
	if include.len() > 3 {
		parser.die(format!("at most 3 --include, got {}", include.len()));
	}

	println!("target       = {target}");
	println!("jobs         = {jobs}");
	println!("project_name = {project_name:?}");
	println!("log          = {log:?}");
	println!("verbose      = {verbose}");
	println!("include      = {include:?}");
	println!("output       = {output:?}");
	println!("inputs       = {inputs:?}");
}
