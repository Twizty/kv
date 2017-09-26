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
//  // Create the event loop that will drive this server
//  let mut core = Core::new().unwrap();
//  let handle = core.handle();
//
//  // Bind the server's socket
//  let addr = "127.0.0.1:12345".parse().unwrap();
//  let listener = TcpListener::bind(&addr, &handle).unwrap();
//
//  // Pull out a stream of sockets for incoming connections
//  let server = listener.incoming().for_each(|(sock, _)| {
//    // Split up the reading and writing parts of the
//    // socket
//    let (reader, writer) = sock.split();
//
//    // A future that echos the data and returns how
//    // many bytes were copied...
//    let bytes_copied = copy(reader, writer);
//
//    // ... after which we'll print what happened
//    let handle_conn = bytes_copied.map(|amt| {
//      println!("wrote {} bytes", amt)
//    }).map_err(|err| {
//      println!("IO error {:?}", err)
//    });
//
//    // Spawn the future as a concurrent task
//    handle.spawn(handle_conn);
//
//    Ok(())
//  });
//
//  // Spin up the server on the event loop
//  core.run(server).unwrap();
}