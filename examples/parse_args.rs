use std::path::PathBuf;

use miniargs::{arg, parse_args, ArgParser};

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
		target: required(arg().short('t').desc("Build target")),
		jobs: default(1, arg().short('j').desc("Parallel jobs").type_name("count")),
		// `project_name` becomes `--project-name` on the command line (all '_' become '-').
		project_name: optional(arg().desc("Project name")),
		log: optional(arg().desc("Log file path")),
		verbose: flag(arg().short('v').desc("Verbose output")),
		include: repeated(arg().short('I').desc("Add include directory")),
		output: positional(arg().desc("Output file")),
		inputs: positional_variadic(arg().desc("Input files")),
	});

	if args.inputs.is_empty() {
		parser.die("at least one input file is required");
	}
	if args.include.len() > 3 {
		parser.die(format!("at most 3 --include, got {}", args.include.len()));
	}

	println!("{args:#?}");
}
