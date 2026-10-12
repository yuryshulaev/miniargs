use miniargs::{arg, enum_choice, ArgParser};

enum_choice! {
	#[derive(Debug, Clone, Copy)]
	pub enum Color {
		Auto => "auto",
		Always => "always",
		Never => "never",
	}
}

fn main() {
	let mut parser = ArgParser::new("enum");
	let color = parser.default("color", Color::Auto, arg().desc("Color mode").type_name_hidden());
	let args = parser.parse(std::env::args_os());
	let color: Color = color.get(&args);
	println!("color = {color}");
}
