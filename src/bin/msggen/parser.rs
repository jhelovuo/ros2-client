#[allow(unused_imports)]
use nom::{
  IResult,
  error::{ParseError, dbg_dmp,},
  branch::alt,
  bytes::complete::{tag, take_while1, take_until, take_till, is_not},
  character::is_alphanumeric,
  character::complete::{char, space0, line_ending, not_line_ending, alphanumeric1,},
  combinator::{map, map_res, value, recognize, eof, opt},
  multi::{many0,many1,},
  sequence::{tuple, pair,delimited, terminated, }
};



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item { 
  Field { type_name: String, field_name: String, default_value: Option<String> },
  Constant{ type_name: String, const_name: String, value: String  },
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

fn type_spec(i: &str) -> IResult<&str, String> {
  map(
    recognize(many1( alt((alphanumeric1, tag("/"), tag("_") )) )),
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

fn comment(i: &str) -> IResult<&str, Comment> {
  map(
    recognize(
      pair( tag("#"), not_line_ending )
    ),
    |s: &str| Comment( s.to_string() )
  )(i)
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
