use std::{ffi::OsString, path::PathBuf};

use miniargs::{parse_args, Arg, ArgParser, Dispatcher};

#[derive(Debug)]
struct AddArgs {
	collection_name: String,
	force: bool,
	files: Vec<PathBuf>,
}

#[derive(Debug)]
struct RemoveArgs {
	collection_name: String,
	force: bool,
}

fn main() {
	let mut dispatcher = Dispatcher::new("myprog");
	dispatcher.command("add", cmd_add, "Add files to a collection");
	dispatcher.command("remove", cmd_remove, "Remove a collection");
	dispatcher.help_command("help", "Print help");
	dispatcher.run(std::env::args_os());
}

fn cmd_add(prog: &str, argv: &[OsString]) {
	let mut parser = ArgParser::new(format!("{prog} add"));
	let args = parse_args!(parser, argv; AddArgs {
		collection_name: required(Arg::new().short('n').desc("Collection to add to")),
		force: flag(Arg::new().short('f').desc("Overwrite existing entries")),
		files: positional_variadic(Arg::new().desc("Files to add")),
	});

	if args.files.is_empty() {
		parser.die("at least one file is required");
	}
	let verb = if args.force { "Overwriting" } else { "Adding" };
	for f in &args.files {
		println!("{verb} {f:?} in collection {:?}", args.collection_name);
	}
}

fn cmd_remove(prog: &str, argv: &[OsString]) {
	let mut parser = ArgParser::new(format!("{prog} remove"));
	let args = parse_args!(parser, argv; RemoveArgs {
		collection_name: required(Arg::new().short('n').desc("Collection to remove")),
		force: flag(Arg::new().short('f').desc("Remove without confirmation")),
	});

	if !args.force {
		eprintln!("error: refusing to remove {:?} without --force", args.collection_name);
		std::process::exit(1);
	}
	println!("Removed collection {:?}", args.collection_name);
}
