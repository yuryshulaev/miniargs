use std::path::PathBuf;

use miniargs::{arg, args, ArgParser};

args! {
	#[derive(Debug)]
	struct Args {
		target: String = required(arg().short('t').desc("Build target")),
		jobs: u32 = default(1, arg().short('j').desc("Parallel jobs").type_name("count")),
		project_name: Option<String> = optional(arg().desc("Project name")),
		log: Option<PathBuf> = optional(arg().desc("Log file path")),
		verbose: bool = flag(arg().short('v').desc("Verbose output")),
		include: Vec<PathBuf> = repeated(arg().short('I').desc("Add include directory")),
		output: PathBuf = positional(arg().desc("Output file")),
		inputs: Vec<PathBuf> = positional_variadic(arg().desc("Input files")),
	}
}

fn main() {
	let args = Args::parse(&mut ArgParser::new("pack"));
	println!("{args:#?}");
}
