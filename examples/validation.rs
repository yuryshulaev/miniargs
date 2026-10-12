use miniargs::{
	arg, args,
	validation::{positive, require},
	ArgParser,
};

fn odd<T>(v: &T) -> Result<(), String>
where
	T: Copy + PartialEq + std::ops::Rem<Output = T> + From<u8>,
{
	require(*v % T::from(2) == T::from(1), "must be odd")
}

args! {
	#[derive(Debug)]
	struct Args {
		jobs: u32 = default(1, arg().short('j').desc("Parallel jobs").type_name("count"))
			.validate(positive).validate(odd),
	}
}

fn main() {
	let args = Args::parse(&mut ArgParser::new("pack"));
	println!("{args:#?}");
}
