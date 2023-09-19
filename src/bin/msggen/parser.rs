#[allow(unused_imports)]
use nom::{
  IResult,
  error::{ParseError, dbg_dmp,},
  branch::alt,
  bytes::complete::{tag, take_while1, take_until, take_till, is_not},
  character::is_alphanumeric,
  character::complete::{char, space0, line_ending, not_line_ending, alphanumeric1, digit1, one_of,},
  combinator::{map, map_res, value, recognize, eof, opt},
  multi::{many0,many1,},
  sequence::{tuple, pair,delimited, terminated, preceded, }
};

use std::str::FromStr;

use super::stringparser::parse_string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment(String);

#[derive(Debug, Clone, PartialEq) ]
pub enum Item { 
  Field { type_name: TypeName, field_name: String, default_value: Option<Value> },
  Constant{ type_name: TypeName, const_name: String, value: Value  },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseTypeName {
  Primitive{ name: String},
  BoundedString{ bound: usize },
  ComplexType {
    package_name: Option<String>,
    type_name: String,
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArraySpecifier {
  Static { size: usize },
  Unbounded,
  Bounded{ bound: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeName {
  base: BaseTypeName,
  array_spec: Option<ArraySpecifier>,
}

#[derive(Debug, Clone, PartialEq,)]
pub enum Value {
  Bool(bool),
  Float(f64), // Also can store a f32
  Int(i64),
  Uint(u64),
  String(Vec<u8>), // ROS does not do Unicode
}



pub fn msg_spec(i: &str) -> IResult<&str, Vec<(Option<Item>, Option<Comment>) >> {
  many0(line)(i)
}

fn line(i: &str) -> IResult<&str, (Option<Item>,Option<Comment>) > {
  terminated(
    pair( alt(( item , just_space)) , opt(comment) ),
    line_ending,
  )(i)

}

fn item(i: &str) -> IResult<&str, Option<Item> > {
  map(
    delimited(space0, alt(( constant, field,  )), space0 ),
    Some
  )(i)
}

fn just_space(i: &str) -> IResult<&str, Option<Item> > {
  value(None, space0)(i)
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

fn type_spec(i: &str) -> IResult<&str, TypeName> {
  //TODO
  map(
    recognize(many1( alt((alphanumeric1, tag("/"), tag("_") )) )),
    |v: &str| TypeName{ base: BaseTypeName::Primitive{ name: v.to_string() } , array_spec: None } 
  )(i)
}

fn identifier(i: &str) -> IResult<&str, String> {
  map(
    alphanumeric1,
    String::from
  )(i)
}

fn value_spec(i: &str) -> IResult<&str, Value> {
  let bool_value = 
    alt(( 
      value(Value::Bool(false), tag("false")),
      value(Value::Bool(true), tag("true")),
    ));
  let float_value = map(float, |f| Value::Float(f) );
  let uint_value = map( digit1, |i| Value::Uint(u64::from_str(i).expect("bad uint")));
  let string_value = map( parse_string, |s:String| Value::String(Vec::from(s)) );

  alt(( bool_value, float_value, /*int_value,*/ uint_value, string_value ))(i)
}

fn comment(i: &str) -> IResult<&str, Comment> {
  map(
    recognize(
      pair( tag("#"), not_line_ending )
    ),
    |s: &str| Comment( s.to_string() )
  )(i)
}

// from "nom" cookbook
fn float(input: &str) -> IResult<&str, f64> {
  map(
    alt((
      // Case one: .42
      recognize(
        tuple((
          char('.'),
          decimal,
          opt( tuple((
            one_of("eE"),
            opt(one_of("+-")),
            decimal
          )))
        ))
      )
      , // Case two: 42e42 and 42.42e42
      recognize(
        tuple((
          decimal,
          opt(preceded(
            char('.'),
            decimal,
          )),
          one_of("eE"),
          opt(one_of("+-")),
          decimal
        ))
      )
      , // Case three: 42. and 42.42
      recognize(
        tuple((
          decimal,
          char('.'),
          opt(decimal)
        ))
      )
    )),
    | f: &str| f64::from_str(f).expect("Failed to parse floating point value.")
    // Failing here means that this nom parser anf f64::from_str disagree on what is a valid float.
  )
  (input)
}

// from "nom" cookbook
fn decimal(input: &str) -> IResult<&str, &str> {
  recognize(
    many1(
      terminated(one_of("0123456789"), many0(char('_')))
    )
  )(input)
}

#[test]
fn comment_test() {
  assert_eq!(comment("#\n"),       Ok(("\n", Comment("#".to_string()) )));
  assert_eq!(comment("# \n"),       Ok(("\n", Comment("# ".to_string())  )));
  assert_eq!(comment("# This message:\n#"),
    Ok(("\n#", Comment("# This message:".to_string())  ))
  );
}

#[test]
fn definition_test() {
  // assert_eq!(line("foo#\n"),       Ok(("#\n", Item::Definition{bytes: "foo".to_string()} )));
  // assert_eq!(line(" bar\n"),       Ok(("\n", Item::Definition{bytes: " bar".to_string()}  )));
  //assert!(line("").is_err() );
}

#[test]
fn item_test() {
  // assert_eq!(idl_item("foo#\n"),       Ok(("#\n", Item::Definition{bytes: "foo".to_string()} )));
  //assert_eq!(idl_item("# \n"),       Ok(("", Item::Comment{bytes: "# \n".to_string()}  )));
}

#[test]
fn spec_test() {
  assert_eq!(msg_spec("\n"),       Ok(("", vec![(None,None)]  )));  
  assert_eq!(msg_spec(""),       Ok(("", vec![] )));
  // assert_eq!(
  //   msg_spec("foo#\n"),       
  //   Ok(("", vec![
  //     Item::Definition{bytes: "foo".to_string()}, 
  //     Item::Comment { bytes: "#\n".to_string() }] )));
  assert_eq!(
    msg_spec("# \n"),
    Ok(("", vec![(None, Some(Comment("# ".to_string()))) ]  ))
  );
}
