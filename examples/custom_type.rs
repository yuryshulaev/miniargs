use std::{ffi::OsStr, fmt};

use miniargs::{Arg, ArgParser, ParseArg};

/// A validated port number. Wraps `u16` so the only way to construct one is
/// through `ParseArg`, which rejects zero.
#[derive(Clone, Copy)]
struct Port(u16);

impl ParseArg for Port {
	fn parse_arg(s: &OsStr) -> Result<Self, String> {
		let s = s.to_str().ok_or("value is not valid UTF-8")?;
		let n: u16 = s.parse().map_err(|e| format!("{e}"))?;
		if n == 0 {
			return Err("port must be greater than zero".into());
		}
		Ok(Port(n))
	}
}

impl fmt::Display for Port {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

fn main() {
	let mut parser = ArgParser::new("server");
	let port = parser.default(
		"port",
		Port(8080),
		Arg::new().short('p').desc("Port to listen on").type_name("port"),
	);
	let args = parser.parse(std::env::args_os());
	let port: Port = port.get(&args);
	println!("port = {port}");
}
