#![feature(read_initializer)]

use futures::{Future, Stream, Poll};
use tokio_core::io::{copy, Io};
use tokio_io;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core,Handle};
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::rc::Rc;
use std::io::{Read, Write, Error, ErrorKind};
use std::time::{Instant};
use super::store::Store;

struct Guard<'a> { buf: &'a mut Vec<u8>, len: usize }

impl<'a> Drop for Guard<'a> {
  fn drop(&mut self) {
    unsafe { self.buf.set_len(self.len); }
  }
}

static EXIT: [u8; 4] = [b'e', b'x', b'i', b't'];
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

struct Handler<R, W> {
  reader: R,
  writer: W,
  buf: Vec<u8>,
  store: Rc<Store>
}

enum Command {
  Get(String),
  Set(String, String),
  Expire(String, Instant),
  Drop(String),
  LGet(String, usize),
  LGetAll(String),
  LInsert(String, usize, String),
  LDrop(String, usize),
  HSet(String, String, String),
  HGet(String, String),
  HGetAll(String)
}

impl<R, W> Future for Handler<R, W>
  where R: Read,
        W: Write,
{
  type Item = u64;
  type Error = Error;

  fn poll(&mut self) -> Poll<u64, Error> {
    loop {
      let result = try_nb!(read_to_block(&mut self.reader, &mut self.buf));
      if self.buf.len() > 3 && self.buf[0..4] == EXIT {
        return Ok(0.into())
      }
      parse_query(&mut self.buf);
      try_nb!(self.writer.write(&self.buf));
      self.buf.clear()
    }
  }
}

fn parse_query(query: &mut Vec<u8>) -> Result<Command, String> {
  let index = query.iter().position(|&r| r == b' ');
  match index {
    Some(u) => {
      let (command_bytes, arguments_bytes) = query.split_at_mut(u);
      let command = String::from_utf8_lossy(command_bytes).into_owned();
      match command.as_ref() {
        "get" => {
          let mut skip_white_spaces = true;
          let mut index_of_ending_argument: usize = 0;
          let mut ending_byte: u8 = 13;
          let mut index_of_beginning_argument: usize = 0;
          let mut skip_bytes: Vec<usize> = Vec::new();
          for (index, byte) in arguments_bytes.iter().enumerate() {
            index_of_ending_argument += 1;
            if *byte == b' ' && skip_white_spaces {
              index_of_beginning_argument += 1;
              continue
            }

            if ending_byte == b'"' && *byte == b'\\' {
              skip_bytes.push(index);
              continue
            }

            if *byte == b'"' && ending_byte == b'"' || *byte == 13 {
              break
            }

            if *byte == b'"' && ending_byte != b'"' {
              ending_byte = b'"'
            }
          }

          let mut argument = String::from_utf8_lossy(
            &mut arguments_bytes[index_of_beginning_argument..index_of_ending_argument-1]
          ).into_owned();

          for x in &skip_bytes {
            argument.remove(*x);
          }
          println!("{:?}", argument);
          return Ok(Command::Get(command))
        },
        _ => return Err("invalid command".to_string())
      }
    },
    None => println!("nothing!")
  }
  Ok(Command::Get("123".to_string()))
}

fn read_to_block<R: Read + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize, Error> {
  let start_len = buf.len();
  let mut g = Guard { len: buf.len(), buf: buf };
  let mut new_write_size = 16;
  let ret;
  loop {
    if g.len == g.buf.len() {
      if new_write_size < DEFAULT_BUF_SIZE {
        new_write_size *= 2;
      }
      unsafe {
        g.buf.reserve(new_write_size);
        g.buf.set_len(g.len + new_write_size);
        r.initializer().initialize(&mut g.buf[g.len..]);
      }
    }

    match r.read(&mut g.buf[g.len..]) {
      Ok(0) => {
        ret = Ok(g.len - start_len);
        break;
      }
      Ok(n) => g.len += n,
      Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
      Err(e) => {
        if e.kind() == ::std::io::ErrorKind::WouldBlock && !is_empty(g.buf) {
          ret = Ok(g.len - start_len);
        } else {
          ret = Err(e);
        }
        break;
      }
    }
  }

  ret
}

fn is_empty(buf: &mut Vec<u8>) -> bool {
  for (i, item) in buf.iter().enumerate() {
    if item != &0 {
      return false
    }
  }

  return true
}

pub struct Server {
  store: Rc<Store>,
  addr: &'static str,
}

impl Server {
  pub fn new(addr: &'static str) -> Server {
    Server {
      store: Rc::new(Store::new()),
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
        buf: Vec::with_capacity(100),
        store: Rc::clone(&  self.store),
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
