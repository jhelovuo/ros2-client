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
  Primitive{ name: String },
  BoundedString{ bound: u64 },
  ComplexType {
    package_name: Option<String>,
    type_name: String,
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArraySpecifier {
  Static { size: u64 },
  Unbounded,
  Bounded{ bound: u64 },
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
  // components of type_spec:
  // array spec: [10] , [<=10], or []
  let array_specifier_inner = 
    alt((
      map(  preceded(tag("<="), uint_value), 
            |bound:u64| ArraySpecifier::Bounded{ bound } ),
      map(  uint_value,
            |size:u64| ArraySpecifier::Static{ size }  ),
      map( space0, |_| ArraySpecifier::Unbounded ),
    ));
  let array_specifier = delimited( char('[') , array_specifier_inner , char(']') );

  // "string<=20"
  let bounded_string = 
    map( 
      preceded(tag("string<="), uint_value),
      |bound:u64| BaseTypeName::BoundedString{ bound }
    );

  let primitive_type =
    map(
      alt(( 
        tag("bool"), tag("byte"), tag("char"),
        tag("float32"), tag("float64"),
        tag("int8"), tag("int16"), tag("int32"), tag("int64"),
        tag("uint8"), tag("uint16"), tag("uint32"), tag("uint64"),
        tag("string")
      )),
      |s:&str| BaseTypeName::Primitive{ name: s.to_string() }
    );

  // "package_name/typename" or "typename"
  let complex_type =
    map(
      pair( opt(terminated(identifier, tag("/") )) , identifier ),
      |(package_name, type_name)| BaseTypeName::ComplexType{ package_name, type_name}
    );

  // type spec:
  let (i, (base, array_spec)) =
    pair(
      alt(( bounded_string, primitive_type , complex_type )),
      opt( array_specifier )
    )(i)?;
  Ok(( i, TypeName{ base, array_spec } ))
}

fn identifier(i: &str) -> IResult<&str, String> {
  map(
    recognize(many1( alt((alphanumeric1, tag("_") )) )),
    String::from
  )(i)
}

fn uint_value(i: &str) -> IResult<&str, u64> {
  map( digit1, |s:&str| u64::from_str(s).expect("bad uint"))(i)
}

fn value_spec(i: &str) -> IResult<&str, Value> {
  let bool_value = 
    alt(( 
      value(Value::Bool(false), tag("false")),
      value(Value::Bool(true), tag("true")),
    ));
  let float_value = map(float, |f| Value::Float(f) );
  let string_value = map( parse_string, |s:String| Value::String(Vec::from(s)) );
  let u_int_value = map( uint_value, |i| Value::Uint(i) );
  let int_value = map(
    preceded(tag("-"), uint_value ),
    |i| Value::Int( -(i as i64) ));

  alt(( bool_value, float_value, int_value, u_int_value, string_value ))(i)
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
