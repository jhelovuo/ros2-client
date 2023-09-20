use std::{io,fs};

use clap::{Arg, Command}; // command line argument processing

mod parser;
mod stringparser;

use parser::{Comment, Item, BaseTypeName, ArraySpecifier, TypeName, Value, };


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

  let msg = parser::msg_spec(&input).unwrap_or_else(|e| panic!("Parse error: {:?}",e));

  println!("{:?}", &msg);
  println!("\n");
  print_struct_definition(&mut io::stdout(), input_file_name , &msg.1)?;

  Ok(())
}


fn print_struct_definition<W:io::Write>(w: &mut W, name: &str, lines: &[(Option<Item>, Option<Comment>)]) 
  -> io::Result<()> 
{
  writeln!(w,"#[derive(Debug, Serialize, Deserialize)]")?;
  writeln!(w, "pub struct {name} {{")?;
  for (item,comment) in lines {
    match (item,comment) {
      (None,None) => writeln!(w,"")?, // empty line
      (None, Some(Comment(c))) => writeln!(w,"  // {c}")?,
      (Some(item), comment_opt) => {
        write!(w,"  ")?;
        match item {
          Item::Field{ type_name, field_name, .. } => {
            let rust_type = translate_type(type_name)?;
            writeln!(w,"{field_name} : {rust_type},")?;
          }
          Item::Constant{const_name,..} => write!(w,"// skipped constant {const_name}")?,
        }

        if let Some(Comment(c)) = comment_opt {
          writeln!(w, "// {c}")?;
        } else {
          writeln!(w,"")?;
        }
      }
    }
  }
  writeln!(w,"}}")?;
  Ok(())
} 

const RUST_BYTESTRING : &'static str = "bstr";

fn translate_type(t: &TypeName) -> io::Result<String> {
  let mut base = String::new();
  match t.base {
    BaseTypeName::Primitive{ ref name} => base.push_str(
      match name.as_str() {
        "bool" => "bool",
        "byte" => "u8",
        "char" => "u8",
        "float32" => "f32",
        "float64" => "f64",
        "int8" => "i8",
        "int16" => "i16",
        "int32" => "i32",
        "int64" => "i64",
        "uint8" => "u8",
        "uint16" => "u16",
        "uint32" => "u32",
        "uint64" => "u64",
        "string" => RUST_BYTESTRING,
        other => panic!("Unexpected primitive type {}", other),
      }
      ),
    BaseTypeName::BoundedString{ ref bound} => base.push_str(RUST_BYTESTRING), // We do not have type to represent boundedness
    BaseTypeName::ComplexType{ ref package_name, ref type_name} => {
      if let Some(pkg) = package_name {
        base.push_str(&pkg); base.push_str("::");
      }
      base.push_str(&type_name);
    }
  }
  //TODO: array specifier
  Ok(base)
}
