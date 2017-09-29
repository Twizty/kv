use futures::{Future, Stream, Poll};
use tokio_core::io::{copy, Io};
use tokio_io;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core,Handle};
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::io::{Read, Write, Error};
use super::store::Store;

struct Handler<R, W> {
  reader: R,
  writer: W,
  buf: Box<[u8]>
}

impl<R, W> Future for Handler<R, W>
  where R: Read,
        W: Write,
{
  type Item = u64;
  type Error = Error;

  fn poll(&mut self) -> Poll<u64, Error> {
    loop {
      let result = try_nb!(self.reader.read(&mut self.buf));
      println!("{:?}", self.buf);
      if self.buf[0] == 1 {
        return Ok(0.into())
      }
      self.buf[0] = b'>';
      try_nb!(self.writer.write(&self.buf));
      self.flush(self.buf.len());
    }
  }
}

impl<R, W> Handler<R, W> {
  fn flush(&mut self, amount: usize) {
    for i in 0..amount {
      self.buf[i] = 0
    }
  }
}

pub struct Server {
  store: Store,
  addr: &'static str,
}

impl Server {
  pub fn new(addr: &'static str) -> Server {
    Server {
      store: Store::new(),
      addr
    }
  }

  pub fn run(&mut self) {
    let mut core = Core::new().unwrap();
    let handler = core.handle();
    let listener = TcpListener::bind(&self.addr.parse().unwrap(), &handler).unwrap();
    let server = listener.incoming().for_each(|(mut sock, _)| {
      // Split up the reading and writing parts of the
      // socket
      let (mut reader, mut writer) = sock.split();

      let mut h = Handler {
        reader,
        writer,
        buf: Box::new([0; 2048])
      };

      let handle_conn = h.map(|v| {
        println!("{}", v)
      }).map_err(|err| {
        println!("{:?}", err)
      });

      // A future that echos the data and returns how
      // many bytes were copied...
//      let bytes_copied = copy(reader, writer);
//
//      // ... after which we'll print what happened
//      let handle_conn = bytes_copied.map(|amt| {
//          println!("wrote {} bytes", amt)
//      }).map_err(|err| {
//          println!("IO error {:?}", err)
//      });

      // Spawn the future as a concurrent task
      handler.spawn(handle_conn);


      Ok(())
    });

    // Spin up the server on the event loop
    core.run(server).unwrap();
  }
}
