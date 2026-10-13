use std::{ffi::OsString, path::PathBuf};

use miniargs::{arg, args, ArgParser, Dispatcher};

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

fn main() {
	let mut dispatcher = Dispatcher::new("myprog");
	dispatcher.command("add", cmd_add, "Add files to a collection");
	dispatcher.command("remove", cmd_remove, "Remove a collection");
	dispatcher.help_command("help", "Print help");
	dispatcher.run();
}

fn cmd_add(prog: &str, argv: &[OsString]) {
	let mut parser = ArgParser::new(format!("{prog} add"));
	let args = AddArgs::parse_from(&mut parser, argv);

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
	let args = RemoveArgs::parse_from(&mut parser, argv);

	if !args.force {
		eprintln!("error: refusing to remove {:?} without --force", args.collection_name);
		std::process::exit(1);
	}
	println!("Removed collection {:?}", args.collection_name);
}
