#![feature(question_mark, quote, rustc_private, associated_type_defaults)]

use syntax::ext::base::{ExtCtxt, MacResult, DummyResult, MacEager};
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{self};
use syntax::ast;
use syntax::ptr::P;

extern crate syntax;

use std::char;

pub trait Ast {
    type E;
    fn ir(&self, cx: &mut ExtCtxt) -> Self::E;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Include {
    path: String
}

#[derive(Debug, PartialEq, Eq)]
pub struct Enum {
    ident: String,
    variants: Vec<String>
}

impl Ast for Enum {
    type E = P<ast::Item>;
    fn ir(&self, cx: &mut ExtCtxt) -> Self::E {
        let mut ident = token::str_to_ident(&self.ident.clone());
        let mut enum_def = ast::EnumDef {
            variants: Vec::new()
        };

        for node in self.variants.iter() {
            let name = token::str_to_ident(&node);
            let span = cx.call_site();
            enum_def.variants.push(ast::Variant {
                node: ast::Variant_ {
                    name: name,
                    attrs: Vec::new(),
                    data: ast::VariantData::Unit(ast::DUMMY_NODE_ID),
                    disr_expr: None
                },
                span: span
            });
        }

        let span = cx.call_site();
        let kind = ast::ItemKind::Enum(enum_def, ast::Generics::default());
        let item = P(ast::Item {
            ident: ident,
            attrs: Vec::new(),
            id: ast::DUMMY_NODE_ID,
            node: kind,
            vis: ast::Visibility::Public,
            span: span
        });

        quote_item!(cx, $item).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Struct {
    ident: String,
    fields: Vec<StructField>
}

#[derive(Debug, PartialEq, Eq)]
pub enum FieldAttribute {
    Optional,
    Required
}

#[derive(Debug, PartialEq, Eq)]
pub struct StructField {
    seq: i16,
    attr: FieldAttribute,
    ty: String,
    ident: String
}

#[derive(Debug, PartialEq, Eq)]
pub struct Ty(pub String);

#[derive(Debug, PartialEq, Eq)]
pub struct Typedef(pub String, pub String);

#[derive(Debug, PartialEq, Eq)]
pub struct Namespace {
    pub lang: String,
    pub module: String
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Keyword {
    Struct,
    Service,
    Enum,
    Namespace,
    Required,
    Optional,
    Oneway,
    Typedef,
    Throws,
    Exception,
    Include,
    Const,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Token {
    Eq,
    Colon,
    SingleQuote,
    Dot,
    Semi,
    At,
    Comma,
    LCurly,
    RCurly,
    LAngle,
    RAngle,
    Number(i16),
    QuotedString(String),
    Ident(String),
    Keyword(Keyword),

    /// Useless comments.
    Comment,
    Whitespace,
    Eof,
    B,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Expected
}

pub struct Parser<'a> {
    buffer: &'a str,
    pos: usize,
    token: Token,
    last_token_eof: bool
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Parser<'a> {
        Parser {
            buffer: input,
            pos: 0,
            token: Token::B,
            last_token_eof: false
        }
    }

    pub fn parse_struct(&mut self) -> Result<Struct, Error> {
        self.expect_keyword(Keyword::Struct)?;

        let ident = self.expect_ident()?;
        let mut fields = Vec::new();

        self.expect(&Token::LCurly)?;

        loop {
            if self.eat(&Token::RCurly) {
                break;
            }

            fields.push(self.parse_struct_field()?);

            if self.eat(&Token::Semi) {
                continue;
            } else {
                break;
            }
        }

        Ok(Struct {
            ident: ident,
            fields: fields
        })
    }

    pub fn parse_struct_field(&mut self) -> Result<StructField, Error> {
        let seq = self.parse_number()?;

        self.expect(&Token::Colon)?;

        let attr = if self.eat_keyword(Keyword::Optional) {
            FieldAttribute::Optional
        } else if self.eat_keyword(Keyword::Required) {
            FieldAttribute::Required
        } else {
            return Err(Error::Expected);
        };

        let ty = self.parse_ident()?;
        let ident = self.parse_ident()?;

        Ok(StructField {
            seq: seq,
            attr: attr,
            ty: ty,
            ident: ident
        })
    }

    pub fn parse_number(&mut self) -> Result<i16, Error> {
        self.skip_b();

        let n = match self.token {
            Token::Number(n) => n,
            _ => return Err(Error::Expected)
        };

        self.bump();
        Ok(n)
    }

    pub fn skip_b(&mut self) {
        if self.token == Token::B {
            self.bump();
        }
    }

    pub fn parse_enum(&mut self) -> Result<Enum, Error> {
        self.expect_keyword(Keyword::Enum)?;

        let ident = self.expect_ident()?;
        let mut variants = Vec::new();

        self.expect(&Token::LCurly)?;

        loop {
            if self.eat(&Token::RCurly) {
                break;
            }

            variants.push(self.parse_ident()?);

            if self.eat(&Token::Comma) {
                continue;
            } else {
                break;
            }
        }

        Ok(Enum {
            ident: ident,
            variants: variants
        })
    }

    pub fn parse_include(&mut self) -> Result<Include, Error> {
        self.expect_keyword(Keyword::Include)?;

        Ok(Include {
            path: self.expect_string()?
        })
    }

    pub fn parse_typedef(&mut self) -> Result<Typedef, Error> {
        self.expect_keyword(Keyword::Typedef)?;

        Ok(Typedef(self.expect_ident()?, self.expect_ident()?))
    }

    pub fn parse_namespace(&mut self) -> Result<Namespace, Error> {
        self.expect_keyword(Keyword::Namespace)?;

        let lang = self.expect_ident()?;
        let module = self.expect_ident()?;

        Ok(Namespace {
            lang: lang,
            module: module
        })
    }

    pub fn expect_string(&mut self) -> Result<String, Error> {
        let val = match self.token {
            Token::QuotedString(ref s) => s.clone(),
            _ => return Err(Error::Expected)
        };

        self.bump();
        Ok(val)
    }

    pub fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), Error> {
        if !self.eat_keyword(keyword) {
            return Err(Error::Expected);
        }

        Ok(())
    }

    pub fn expect(&mut self, token: &Token) -> Result<Token, Error> {
        if !self.eat(token) {
            return Err(Error::Expected);
        } else {
            Ok(self.token.clone())
        }
    }

    pub fn parse_ident(&mut self) -> Result<String, Error> {
        if self.token == Token::B {
            self.bump();
        }

        let i = match self.token {
            Token::Ident(ref s) => s.clone(),
            _ => return Err(Error::Expected)
        };

        self.bump();
        Ok(i)
    }

    pub fn expect_ident(&mut self) -> Result<String, Error> {
        let ident = match self.token {
            Token::Ident(ref s) => s.clone(),
            _ => return Err(Error::Expected)
        };

        self.bump();
        Ok(ident)
    }

    pub fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        self.eat(&Token::Keyword(keyword))
    }

    fn next_char(&self) -> char {
        self.buffer[self.pos..].chars().next().unwrap()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.buffer[self.pos ..].starts_with(s)
    }

    fn eof(&self) -> bool {
        self.pos >= self.buffer.len()
    }

    fn consume_char(&mut self) -> char {
        let mut iter = self.buffer[self.pos..].char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        return cur_char;
    }

    fn next_token(&mut self) -> Token {
        if self.eof() {
            return Token::Eof;
        }

        let ch = self.consume_char();

        match ch {
            ':' => Token::Colon,
            '.' => Token::Dot,
            ';' => Token::Semi,
            ',' => Token::Comma,
            '"' => {
                let val = self.consume_while(|c| c != '"' || c != '\"');
                self.consume_char();
                Token::QuotedString(val)
            },
            '=' => Token::Eq,
            '{' => Token::LCurly,
            '}' => Token::RCurly,
            '<' => Token::LAngle,
            '>' => Token::RAngle,
            '0'...'9' => {
                let mut val = self.consume_while(|c| match c {
                    '0'...'9' => true,
                    _ => false
                });

                val = format!("{}{}", ch, val);

                Token::Number(val.parse().unwrap())
            },
            '/' | '#' => {
                if self.next_char() == '/' || ch == '#' {
                    self.consume_while(|c| c != '\n' && c != '\r');
                    return Token::Comment
                } else if self.next_char() == '*' {
                    self.consume_char();
                    loop {
                        self.consume_while(|c| c != '*');
                        self.consume_char();

                        if self.next_char() == '/' {
                            break;
                        }
                    }

                    // Consume the following '/' because we just did a lookahead previously.
                    self.consume_char();
                    return Token::Comment
                }

                Token::Eof
            },
            c if c.is_whitespace() => {
                self.consume_whitespace();
                self.next_token()
            },
            // identifier
            'a'...'z' | 'A'...'Z' | '_' => {
                let mut ident = self.consume_while(|c| match c {
                    'a'...'z' => true,
                    'A'...'Z' => true,
                    '_' => true,
                    '0'...'9' => true,
                    _ => false
                });

                ident = format!("{}{}", ch, ident);

                match &*ident {
                    "namespace" => return Token::Keyword(Keyword::Namespace),
                    "struct" => return Token::Keyword(Keyword::Struct),
                    "enum" => return Token::Keyword(Keyword::Enum),
                    "service" => return Token::Keyword(Keyword::Service),
                    "optional" => return Token::Keyword(Keyword::Optional),
                    "required" => return Token::Keyword(Keyword::Required),
                    "throws" => return Token::Keyword(Keyword::Throws),
                    "oneway" => return Token::Keyword(Keyword::Oneway),
                    "typedef" => return Token::Keyword(Keyword::Typedef),
                    "exception" => return Token::Keyword(Keyword::Exception),
                    "include" => return Token::Keyword(Keyword::Include),
                    "const" => return Token::Keyword(Keyword::Const),
                    _ => Token::Ident(ident)
                }
            },
            _ => Token::Eof
        }
    }

    pub fn eat(&mut self, token: &Token) -> bool {
        if self.token == Token::B {
            self.bump();
        }

        if self.token == *token {
            self.bump();
            true
        } else {
            false
        }
    }

    fn consume_while<F>(&mut self, test: F) -> String
        where F: Fn(char) -> bool {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }
        return result;
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    pub fn bump(&mut self) {
        if self.last_token_eof {
            panic!("attempted to bump past eof.");
        }

        if self.token == Token::Eof {
            self.last_token_eof = true;
        }

        self.token = self.next_token();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eof_token() {
        let mut parser = Parser::new("");
        assert_eq!(parser.eof(), true);
        assert_eq!(parser.next_token(), Token::Eof);
    }

    #[test]
    fn colon_token() {
        let mut parser = Parser::new(":");
        assert_eq!(parser.next_token(), Token::Colon);
    }

    #[test]
    fn dot_token() {
        let mut parser = Parser::new(".");
        assert_eq!(parser.next_token(), Token::Dot);
    }

    #[test]
    fn equal_token() {
        let mut parser = Parser::new("=");
        assert_eq!(parser.next_token(), Token::Eq);
    }

    #[test]
    fn comma_token() {
        let mut parser = Parser::new(",");
        assert_eq!(parser.next_token(), Token::Comma);
    }

    #[test]
    fn curly_token() {
        let mut parser = Parser::new("{}");
        assert_eq!(parser.next_token(), Token::LCurly);
        assert_eq!(parser.next_token(), Token::RCurly);
    }

    #[test]
    fn angle_token() {
        let mut parser = Parser::new("<>");
        assert_eq!(parser.next_token(), Token::LAngle);
        assert_eq!(parser.next_token(), Token::RAngle);
    }

    #[test]
    fn semi_token() {
        let mut parser = Parser::new(";");
        assert_eq!(parser.next_token(), Token::Semi);
    }

    #[test]
    #[should_panic]
    fn whitespace_token() {
        let mut parser = Parser::new(" ");
        assert_eq!(parser.next_token(), Token::Whitespace);
    }

    #[test]
    fn whitespace_grab_token() {
        let mut parser = Parser::new("     >");
        assert_eq!(parser.next_token(), Token::RAngle);
    }

    #[test]
    fn comment_token() {
        let mut parser = Parser::new("<//foobar\n:");
        assert_eq!(parser.next_token(), Token::LAngle);
        assert_eq!(parser.next_token(), Token::Comment);
        assert_eq!(parser.next_token(), Token::Colon);
    }

    #[test]
    fn multi_comment_token() {
        let mut parser = Parser::new("</*\n
                                     fofoo*/>");
        assert_eq!(parser.next_token(), Token::LAngle);
        assert_eq!(parser.next_token(), Token::Comment);
        assert_eq!(parser.next_token(), Token::RAngle);
    }

    #[test]
    fn hash_comment_token() {
        let mut parser = Parser::new("<#foobar\n:");
        assert_eq!(parser.next_token(), Token::LAngle);
        assert_eq!(parser.next_token(), Token::Comment);
        assert_eq!(parser.next_token(), Token::Colon);
    }

    #[test]
    fn ident_token() {
        assert_eq!(Parser::new("foobar").next_token(), Token::Ident("foobar".to_string()));
        assert_eq!(Parser::new("foobar123").next_token(), Token::Ident("foobar123".to_string()));
        assert_eq!(Parser::new("foobar_123").next_token(), Token::Ident("foobar_123".to_string()));
        assert_eq!(Parser::new("_FFF").next_token(), Token::Ident("_FFF".to_string()));
    }

    #[test]
    fn quoted_string_token() {
        assert_eq!(Parser::new("\"hello world 12338383\"").next_token(), Token::QuotedString("hello world 12338383".to_string()));
    }

    #[test]
    #[should_panic]
    fn fail_ident_token() {
        assert_eq!(Parser::new("1foobar").next_token(), Token::Ident("1foobar".to_string()));
    }

    #[test]
    fn keywords_token() {
        assert_eq!(Parser::new("oneway").next_token(), Token::Keyword(Keyword::Oneway));
        assert_eq!(Parser::new("exception").next_token(), Token::Keyword(Keyword::Exception));
        assert_eq!(Parser::new("struct").next_token(), Token::Keyword(Keyword::Struct));
        assert_eq!(Parser::new("enum").next_token(), Token::Keyword(Keyword::Enum));
        assert_eq!(Parser::new("namespace").next_token(), Token::Keyword(Keyword::Namespace));
        assert_eq!(Parser::new("service").next_token(), Token::Keyword(Keyword::Service));
        assert_eq!(Parser::new("throws").next_token(), Token::Keyword(Keyword::Throws));
        assert_eq!(Parser::new("typedef").next_token(), Token::Keyword(Keyword::Typedef));
        assert_eq!(Parser::new("optional").next_token(), Token::Keyword(Keyword::Optional));
        assert_eq!(Parser::new("required").next_token(), Token::Keyword(Keyword::Required));
        assert_eq!(Parser::new("const").next_token(), Token::Keyword(Keyword::Const));
    }

    #[test]
    fn eat_token() {
        let mut p = Parser::new(":");
        assert_eq!(p.eat(&Token::Colon), true);
    }

    #[test]
    fn eat_keywords() {
        let mut p = Parser::new("oneway");
        assert_eq!(p.eat_keyword(Keyword::Oneway), true);
    }

    #[test]
    fn parse_namespace() {
        let mut p = Parser::new("namespace rust foobar");
        let ns = p.parse_namespace().unwrap();
        assert_eq!(&*ns.lang, "rust");
        assert_eq!(&*ns.module, "foobar");
    }

    #[test]
    fn parse_namespaace() {
        let mut p = Parser::new("namespace rust foobar");
        let ns = p.parse_namespace().unwrap();
        assert_eq!(&*ns.lang, "rust");
        assert_eq!(&*ns.module, "foobar");
    }

    #[test]
    fn parse_include() {
        let mut p = Parser::new("include \"./../include.thrift\"");
        let ns = p.parse_include().unwrap();
        assert_eq!(&*ns.path, "./../include.thrift");
    }

    #[test]
    fn parse_typedef() {
        let mut p = Parser::new("typedef i32 MyInteger");
        let def = p.parse_typedef().unwrap();
        assert_eq!(&*def.0, "i32");
        assert_eq!(&*def.1, "MyInteger");
    }

    #[test]
    fn parse_empty_enum() {
        let mut p = Parser::new("enum FooBar {}");
        let def = p.parse_enum().unwrap();
        assert_eq!(&*def.ident, "FooBar");
        assert_eq!(def.variants.len(), 0);
    }

    #[test]
    fn parse_one_variant_enum() {
        let mut p = Parser::new("enum Hello { ONE }");
        let def = p.parse_enum().unwrap();
        assert_eq!(&*def.ident, "Hello");
        assert_eq!(def.variants.len(), 1);
        assert_eq!(&*def.variants[0], "ONE");
    }

    #[test]
    fn parse_multi_variant_enum() {
        let mut p = Parser::new("enum Hello { ONE, TWO }");
        let def = p.parse_enum().unwrap();
        assert_eq!(&*def.ident, "Hello");
        assert_eq!(def.variants.len(), 2);
        assert_eq!(&*def.variants[0], "ONE");
        assert_eq!(&*def.variants[1], "TWO");
    }

    #[test]
    fn parse_empty_struct() {
        let mut p = Parser::new("struct FooBar {}");
        let def = p.parse_struct().unwrap();
        assert_eq!(&*def.ident, "FooBar");
        assert_eq!(def.fields.len(), 0);
    }

    #[test]
    fn parse_struct_w_field() {
        let mut p = Parser::new("struct FooBar { 1: required i32 mycat }");
        let def = p.parse_struct().unwrap();
        assert_eq!(&*def.ident, "FooBar");
        assert_eq!(def.fields.len(), 1);
    }

    #[test]
    fn parse_struct_w_multi_field() {
        let mut p = Parser::new("struct FooBar { 1: required i32 mycat; 2: required i32 two }");
        let def = p.parse_struct().unwrap();
        assert_eq!(&*def.ident, "FooBar");
        assert_eq!(def.fields.len(), 2);
    }

    #[test]
    fn parse_struct_field_optional() {
        let mut p = Parser::new("1: optional i32 foobar");
        let def = p.parse_struct_field().unwrap();
        assert_eq!(&*def.ident, "foobar");
        assert_eq!(&*def.ty, "i32");
        assert_eq!(def.seq, 1);
        assert_eq!(def.attr, FieldAttribute::Optional);
    }

    #[test]
    fn parse_struct_field_required() {
        let mut p = Parser::new("1: required i32 foobar");
        let def = p.parse_struct_field().unwrap();
        assert_eq!(&*def.ident, "foobar");
        assert_eq!(&*def.ty, "i32");
        assert_eq!(def.seq, 1);
        assert_eq!(def.attr, FieldAttribute::Required);
    }
}
