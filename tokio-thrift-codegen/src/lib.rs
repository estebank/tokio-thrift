extern crate handlebars;
extern crate rustc_serialize;
extern crate tokio_thrift_parser;

use std::io::{self, Write};
use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json};
use handlebars::{Handlebars, RenderError, RenderContext, Helper, Context, JsonRender};
use tokio_thrift_parser::{Ty, Namespace, Parser, Keyword};


#[derive(Debug)]
pub enum Error {
    Other,
    IO(io::Error),
    Parser(tokio_thrift_parser::Error),
    Eof,
}

impl From<io::Error> for Error {
    fn from(val: io::Error) -> Error {
        Error::IO(val)
    }
}

impl From<tokio_thrift_parser::Error> for Error {
    fn from(val: tokio_thrift_parser::Error) -> Error {
        Error::Parser(val)
    }
}

pub fn find_rust_namespace(parser: &mut Parser) -> Result<Namespace, Error> {
    loop {
        let ns = parser.parse_namespace()?;

        if &*ns.lang == "rust" {
            return Ok(ns);
        } else {
            continue;
        }
    }
}

// define a custom helper
fn helper_ty_to_protocol(_: &Context,
                         h: &Helper,
                         _: &Handlebars,
                         rc: &mut RenderContext)
                         -> Result<(), RenderError> {
    let param = try!(h.param(0)
        .ok_or(RenderError::new("Param 0 is required for to_protocol helper.")));
    let rendered = param.value().render();
    let ty = Ty::from(rendered);
    let ret = ty.to_protocol();
    try!(rc.writer.write(ret.as_bytes()));
    Ok(())
}

fn helper_ty_to_rust(_: &Context,
                     h: &Helper,
                     _: &Handlebars,
                     rc: &mut RenderContext)
                     -> Result<(), RenderError> {
    let param = try!(h.param(0).ok_or(RenderError::new("Param 0 is required for to_rust helper.")));
    let rendered = param.value().render();
    let ty = Ty::from(rendered);
    let ret = ty.to_string();
    try!(rc.writer.write(ret.as_bytes()));
    Ok(())
}

fn helper_ty_expr(_: &Context,
                  h: &Helper,
                  _: &Handlebars,
                  rc: &mut RenderContext)
                  -> Result<(), RenderError> {
    let param = try!(h.param(0).ok_or(RenderError::new("Param 0 is required for expr helper.")));
    let rendered = param.value().render();
    let ty = Ty::from(rendered);
    let expr = match ty {
        Ty::String => "de.deserialize_str()",
        Ty::I32 => "de.deserialize_i32()",
        Ty::I16 => "de.deserialize_i16()",
        Ty::I64 => "de.deserialize_i64()",
        Ty::Bool => "de.deserialize_bool()",
        _ => panic!("Unexpected type to deserialize_arg: {:?}.", ty),
    };
    try!(rc.writer.write(expr.as_bytes()));
    Ok(())
}

macro_rules! static_register {
    ($handlebar: expr, $name: expr, $file: expr) => {
        $handlebar.register_template_string($name, include_str!($file).to_string()).expect("tokio_thrift internal error: failed to register template");
    }
}

macro_rules! static_register_files {
    ($handlebar: expr $(, $name: expr)*) => {
        $(static_register!($handlebar, $name, concat!($name, ".hbs"));)*
    }
}


pub fn compile(parser: &mut Parser, wr: &mut Write) -> Result<(), Error> {
    let mut handlebars = Handlebars::new();
    static_register_files!(handlebars, "base", "service", "service_client", "service_server", "struct", "enum", "typedef", "const", "method");

    handlebars.register_helper("expr", Box::new(helper_ty_expr));
    handlebars.register_helper("to_protocol", Box::new(helper_ty_to_protocol));
    handlebars.register_helper("to_rust", Box::new(helper_ty_to_rust));


    let data: BTreeMap<String, Json> = BTreeMap::new();
    try!(write!(wr,
                "{}",
                handlebars.render("base", &data).expect("faled to render base file")));

    loop {
        let mut data: BTreeMap<String, Json> = BTreeMap::new();
        if parser.lookahead_keyword(Keyword::Enum) {
            let enum_ = parser.parse_enum()?;
            let json = json::encode(&enum_)
                .ok()
                .and_then(|s| Json::from_str(&s).ok())
                .expect("internal error");
            data.insert("enum".to_string(), json);
            write!(wr,
                   "{}",
                   handlebars.render("enum", &data).expect("internal error"))
                .expect("faled to render enum");
        } else if parser.lookahead_keyword(Keyword::Struct) {
            let struct_ = parser.parse_struct()?;
            let json = json::encode(&struct_)
                .ok()
                .and_then(|s| Json::from_str(&s).ok())
                .expect("internal error");
            data.insert("struct".to_string(), json);
            write!(wr,
                   "{}",
                   handlebars.render("struct", &data).expect("internal error"))
                .expect("faled to render struct");
        } else if parser.lookahead_keyword(Keyword::Typedef) {
            let typedef = parser.parse_typedef()?;
            let json = json::encode(&typedef)
                .ok()
                .and_then(|s| Json::from_str(&s).ok())
                .expect("internal error");
            data.insert("typedef".to_string(), json);
            println!("{:?}", data);
            write!(wr,
                   "{}",
                   handlebars.render("typedef", &data).expect("internal error"))
                .expect("faled to render typedef");
        } else if parser.lookahead_keyword(Keyword::Const) {
            let const_ = parser.parse_const()?;
            let json = json::encode(&const_)
                .ok()
                .and_then(|s| Json::from_str(&s).ok())
                .expect("internal error");
            data.insert("const".to_string(), json);
            println!("{:?}", data);
            write!(wr,
                   "{}",
                   handlebars.render("const", &data).expect("internal error"))
                .expect("faled to render const_");
        } else if parser.lookahead_keyword(Keyword::Service) {
            let service = parser.parse_service()?;
            let json = json::encode(&service)
                .ok()
                .and_then(|s| Json::from_str(&s).ok())
                .expect("internal error");
            data.insert("service".to_string(), json);
            write!(wr,
                   "{}",
                   handlebars.render("service", &data).expect("internal error"))
                .expect("faled to render service");
            write!(wr,
                   "{}",
                   handlebars.render("service_client", &data).expect("internal error"))
                .expect("faled to render client of service");
            write!(wr,
                   "{}",
                   handlebars.render("service_server", &data).expect("internal error"))
                .expect("faled to render server of service");
        } else {
            break;
        }
    }

    Ok(())
}