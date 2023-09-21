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
      .author("Juhana Helovuo <juhana.helovuo@atostek.com>")
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
  // assume that first we have only constants and comments
  let is_not_field = |i:&Item| {match i { Item::Field{..} => false, _  => true, }};

  let not_yet = lines.iter().take_while(|p| p.0.as_ref().map_or(true, is_not_field));
  let got_field = lines.iter().skip_while(|p| p.0.as_ref().map_or(true, is_not_field));

  for (item,comment) in not_yet {
    match (item,comment) {
      (None,None) => writeln!(w,"")?, // empty line
      (None, Some(Comment(c))) => writeln!(w,"// {c}")?,
      (Some(item), comment_opt) => {
        match item {
          Item::Field{ .. } => panic!("Why am i here?"),
          Item::Constant{type_name, const_name, value} => {
            let rust_type = translate_type(type_name)?;
            let rust_value = translate_value(value);
            write!(w, "pub const {const_name} : {rust_type} = {rust_value};")?;
          }
        }

        if let Some(Comment(c)) = comment_opt {
          writeln!(w, "// {c}")?;
        } else {
          writeln!(w,"")?;
        }
      }
    }
  }


  writeln!(w,"#[derive(Debug, Serialize, Deserialize)]")?;
  writeln!(w, "pub struct {name} {{")?;
  for (item,comment) in got_field {
    match (item,comment) {
      (None,None) => writeln!(w,"")?, // empty line
      (None, Some(Comment(c))) => writeln!(w,"  // {c}")?,
      (Some(item), comment_opt) => {
        write!(w,"  ")?;
        match item {
          Item::Field{ type_name, field_name, .. } => {
            let rust_type = translate_type(type_name)?;
            write!(w,"{field_name} : {rust_type}, ")?;
          }
          Item::Constant{const_name,..} => write!(w,"// skipped constant {const_name} in the middle of struct")?,
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

const RUST_BYTESTRING : &'static str = "BString";

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
    BaseTypeName::BoundedString{ .. } => base.push_str(RUST_BYTESTRING), // We do not have type to represent boundedness
    BaseTypeName::ComplexType{ ref package_name, ref type_name} => {
      if let Some(pkg) = package_name {
        base.push_str(&pkg); base.push_str("::");
      }
      base.push_str(&type_name);
    }
  }

  match t.array_spec {
    None => {},
    Some(ArraySpecifier::Static{size}) => {
      base = format!("[{};{}]", base, size);
    }
    Some(ArraySpecifier::Unbounded) |
    Some(ArraySpecifier::Bounded{..}) => {
      base = format!("Vec<{}>", base);
    }
  }

  Ok(base)
}

fn translate_value(v: &Value) -> String {
  match v {
    Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
    Value::Float(f) => format!("{f}"),
    Value::Int(i) => format!("{i}"),
    Value::Uint(u) => format!("{u}"),
    Value::String(v) => String::from_utf8(v.to_vec()).unwrap(),
  }
}
