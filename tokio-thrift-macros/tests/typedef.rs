#![feature(plugin)]
#![plugin(tokio_thrift_macros)]

extern crate futures;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate tokio_thrift;

thrift_file!("tests/typedef.thrift");

#[test]
fn typedef_available() {
    
}
