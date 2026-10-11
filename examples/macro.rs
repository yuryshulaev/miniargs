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
