use std::ffi::OsStr;

use miniargs::{Arg, ArgParser, ParseArg};

#[derive(Debug, Clone, Copy)]
enum Color {
	Auto,
	Always,
	Never,
}

impl ParseArg for Color {
	fn parse_arg(s: &OsStr) -> Result<Self, String> {
		match s.to_str() {
			Some("auto") => Ok(Color::Auto),
			Some("always") => Ok(Color::Always),
			Some("never") => Ok(Color::Never),
			Some(other) => Err(format!("expected auto, always, or never, got '{other}'")),
			None => Err("value is not valid UTF-8".to_string()),
		}
	}
}

impl std::fmt::Display for Color {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Color::Auto => write!(f, "auto"),
			Color::Always => write!(f, "always"),
			Color::Never => write!(f, "never"),
		}
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
