use nom::{
  IResult,
  error::{ParseError, dbg_dmp,},
  branch::alt,
  bytes::complete::{tag, take_while1, take_until, take_till, is_not},
  character::complete::{char, space0, line_ending, not_line_ending, alphanumeric1,},
  combinator::{map, map_res, value, recognize, eof},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item { 
  Comment { bytes: String},
  Field { type_name: String, field_name: String, default_value: Option<String> },
  Constant{ type_name: String, const_name: String, value: String  },
  Whitespace,
}

fn field(i: &str) -> IResult<&str, Item> {
  let (i,type_name) = type_spec(i)?;
  let (i,_) = space0(i)?;
  let (i,field_name) = identifier(i)?;
  Ok(( i, Item::Field{ type_name, field_name, default_value:None} ))
}

fn constant(i: &str) -> IResult<&str, Item> {
  let (i,type_name) = type_spec(i)?;
  let (i,_) = space0(i)?;
  let (i,const_name) = identifier(i)?;
  let (i,_) = space0(i)?;
  let (i,_) = tag("=")(i)?;
  let (i,_) = space0(i)?;
  let (i,value) = value_spec(i)?;
  Ok(( i, Item::Constant{ type_name, const_name, value } ))
}

fn type_spec(i: &str) -> IResult<&str, String> {
  map(
    alphanumeric1,
    String::from
  )(i)
}

fn identifier(i: &str) -> IResult<&str, String> {
  map(
    alphanumeric1,
    String::from
  )(i)
}

fn value_spec(i: &str) -> IResult<&str, String> {
  map(
    alphanumeric1,
    String::from
  )(i)
}


fn idl_specification(i: &str) -> IResult<&str, Vec<Item>> {
  many0(idl_item)(i)
}

fn idl_item(i: &str) -> IResult<&str, Item> {
  alt(( comment, empty_line, line ))(i)
}

fn empty_line(i: &str) -> IResult<&str, Item> {
  value(
    Item::Whitespace,
    terminated( space0, line_ending )
  )(i)
}

fn line(i: &str) -> IResult<&str, Item> {
  delimited(space0, alt(( constant, field )), space0 )(i)
  // map( 
  //   take_while1(|c| c != '\n' && c != '#') ,
  //   |s: &str| Item::Definition{ bytes: s.to_string() }
  // )(i)
}

fn comment(i: &str) -> IResult<&str, Item> {
  map(
    recognize(
      tuple(( tag("#"), not_line_ending, line_ending ))
    ),
    |s: &str| Item::Comment{ bytes: s.to_string() }
  )(i)
}




#[test]
fn empty_test() {
  assert_eq!(empty_line("\n"),       Ok(("", Item::Whitespace)));
  assert_eq!(empty_line("\n\n"),       Ok(("\n", Item::Whitespace)));
  assert_eq!(empty_line("\n \n"),       Ok((" \n", Item::Whitespace)));
  assert_eq!(empty_line(" \n"),       Ok(("", Item::Whitespace)));
}

#[test]
fn comment_test() {
  assert_eq!(comment("#\n"),       Ok(("", Item::Comment{bytes: "#\n".to_string()} )));
  assert_eq!(comment("# \n"),       Ok(("", Item::Comment{bytes: "# \n".to_string()}  )));
  assert_eq!(comment("# This message:\n#"),
    Ok(("#", Item::Comment{bytes: "# This message:\n".to_string()}  )));
}

#[test]
fn definition_test() {
  // assert_eq!(line("foo#\n"),       Ok(("#\n", Item::Definition{bytes: "foo".to_string()} )));
  // assert_eq!(line(" bar\n"),       Ok(("\n", Item::Definition{bytes: " bar".to_string()}  )));
  assert!(line("").is_err() );
}

#[test]
fn item_test() {
  // assert_eq!(idl_item("foo#\n"),       Ok(("#\n", Item::Definition{bytes: "foo".to_string()} )));
  assert_eq!(idl_item("# \n"),       Ok(("", Item::Comment{bytes: "# \n".to_string()}  )));
}

#[test]
fn spec_test() {
  assert_eq!(idl_specification("\n"),       Ok(("", vec![Item::Whitespace]  )));  
  assert_eq!(idl_specification(""),       Ok(("", vec![] )));
  // assert_eq!(
  //   idl_specification("foo#\n"),       
  //   Ok(("", vec![
  //     Item::Definition{bytes: "foo".to_string()}, 
  //     Item::Comment { bytes: "#\n".to_string() }] )));
  assert_eq!(
    idl_specification("# \n"),
    Ok(("", vec![Item::Comment{bytes: "# \n".to_string()}]  )));
}
