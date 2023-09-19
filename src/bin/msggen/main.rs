use std::{io,fs};

use clap::{Arg, Command}; // command line argument processing

mod parser;

use parser::{Comment, Item, msg_spec};

fn main() -> io::Result<()> {
  println!("msggen");

  let arg_matches =
    Command::new("msggen")
      .version("0.0.1")
      .author("Juhana Helovuo <juhe@iki.fi>")
      .about("ros2-client IDL compiler for Rust")
      .arg(Arg::new("input")
        .short('i')
        .help("Input .msg file name")
        .value_name("file")
      )
      .get_matches();

  let input_file_name = arg_matches.get_one::<String>("input").map(String::as_str)
    .unwrap_or("-");

  let input_file = fs::File::open(input_file_name)?;

  let input = io::read_to_string(input_file)?;

  println!("{:?}", msg_spec(&input) );

  Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BaseTypeName {
  PrimitiveType{name: String},
  BoundedString{ bound: usize },
  ComplexType {
    package_name: Option<String>,
    type_name: String,
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArraySpecifier {
  Static { size: usize },
  Unbounded,
  Bounded{ bound: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TypeName {
  base: BaseTypeName,
  array_spec: ArraySpecifier,
}

#[derive(Debug, Clone, PartialEq,)]
enum Value {
  Bool(bool),
  Byte(u8),
  Char(u8), // not more than 8 bits
  Float(f64), // Also can store a f32
  Int(i64),
  Uint(u64),
  String(Vec<u8>), // ROS does not do Unicode
}



