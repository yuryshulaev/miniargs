use miniargs::{all_variants, Arg, ArgParser, Choice};

#[derive(Debug, Clone, Copy)]
enum Color {
	Auto,
	Always,
	Never,
}

impl Choice for Color {
	const ALL: &'static [Color] = all_variants!(Color; Auto, Always, Never);

	fn name(&self) -> &'static str {
		match self {
			Color::Auto => "auto",
			Color::Always => "always",
			Color::Never => "never",
		}
	}
}

impl std::fmt::Display for Color {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.name())
	}
}

fn main() {
	let mut parser = ArgParser::new("enum");
	let color =
		parser.default("color", Color::Auto, Arg::new().desc("Color mode").type_name_hidden());
	let args = parser.parse(std::env::args_os());
	let color: Color = color.get(&args);
	println!("color = {color}");
}
