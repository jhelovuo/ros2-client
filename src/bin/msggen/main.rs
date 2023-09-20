use std::{io,fs};

use clap::{Arg, Command}; // command line argument processing

mod parser;
mod stringparser;


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

  println!("{:?}", parser::msg_spec(&input) );

  Ok(())
}


