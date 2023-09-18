use nom::{
  IResult,
  error::ParseError,
  branch::alt,
  bytes::complete::{tag, take_while_m_n, take_until, take_till, is_not},
  character::complete::{char, multispace0, newline, },
  combinator::{map, map_res, value, recognize},
  multi::many0,
  sequence::{tuple, pair,delimited, terminated, }
};

use clap::{Arg, ArgMatches, Command}; // command line argument processing

use std::{io,fs};


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

  println!("{:?}", idl_specification(&input) );

  Ok(())
}

#[derive(Debug, Clone)]
pub enum Item { 
  Comment { bytes: String},
  Definition { bytes: String},
  Whitespace,
}


fn idl_specification(i: &str) -> IResult<&str, Vec<Item>> {
  many0(idl_item)(i)
}

fn idl_item(i: &str) -> IResult<&str, Item> {
  alt(( line, comment, empty_line ))(i)
}

fn empty_line(i: &str) -> IResult<&str, Item> {
  value(
    Item::Whitespace,
    recognize(
      pair( multispace0, newline )
    )
  )(i)
}



pub fn line(i: &str) -> IResult<&str, Item> {
  map( 
    recognize( 
      terminated(
        take_till(|c| c == '\n' || c == '#') ,
        alt(( comment, empty_line ))
      )
    ),
    |s: &str| Item::Definition{ bytes: s.to_string() }
  )(i)
}

pub fn comment(i: &str) -> IResult<&str, Item> {
  map(
    recognize(
      tuple(( tag("#"), is_not("\n"), newline ))
    ),
    |s: &str| Item::Comment{ bytes: s.to_string() }
  )(i)
}

