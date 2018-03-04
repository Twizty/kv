#![feature(read_initializer)]
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate tokio_core;
extern crate tokio_proto;
#[macro_use]
extern crate tokio_io;

use futures::{Future, Stream};
use tokio_core::io::{copy, Io};
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core,Handle};
use std::collections::hash_map::Entry;

mod server;
mod store;
use server::Server;

fn main() {
  let mut s = Server::new("127.0.0.1:12345");
  s.run()
}