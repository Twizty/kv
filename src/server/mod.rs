#![feature(read_initializer)]

use futures::{Future, Stream, Poll};
use serde_json;
use tokio_core::io::{copy, Io};
use tokio_io;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core,Handle};
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::rc::Rc;
use serde::{Serialize, Serializer};
use std::cell::{RefCell, RefMut};
use std::time::Duration;
use std::borrow::Cow;
use std::num::ParseIntError;
use std::collections::HashMap;
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
static EMPTY_LINE: [u8; 1] = [b'\n'];
static MARSHALLING_ERROR: [u8; 15] = [b'c', b'a', b'n', b'n', b'o', b't', b' ', b'm', b'a', b'r', b's', b'h', b'a', b'l', b'\n'];
static EMPTY_KEY: &'static str = "Key is empty";
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

#[derive(Serialize)]
enum Status {
  Ok,
  Error,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Res<'a> {
  StringResult(&'a String),
  ListResult(&'a Vec<String>),
  HashResult(&'a HashMap<String, String>),
  RawResult(&'static str)
}

#[derive(Serialize)]
struct Response<'a> {
  status:  Status,
  message: Option<Res<'a>>,
}

struct Handler<R, W> {
  reader: R,
  writer: W,
  buf: Vec<u8>,
  skip_bytes: Vec<usize>,
  store: Rc<RefCell<Store>>
}

#[derive(Debug)]
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
      match parse_query(&mut self.buf, &mut self.skip_bytes) {
        Ok(c) => {
          match self.process_command(c) {
            Ok(r) => {
              self.writer.write(&r.into_bytes());
              self.writer.write(&EMPTY_LINE);
            },
            Err(e) => {
              println!("{:?}", e);
              self.writer.write(&MARSHALLING_ERROR);
            },
          }
        },
        Err(e) => {
          println!("{:?}", e);
          self.writer.write(&MARSHALLING_ERROR);
        }
      };
      self.buf.clear();
      self.skip_bytes.clear();
    }
  }
}

impl<R, W> Handler<R, W> {
  fn process_command(&mut self, c: Command) -> serde_json::Result<String> {
    let mut store = self.store.borrow_mut();
    match c {
      Command::Get(s) => {
        match store.get(s) {
          Some(ref mut v) => {
            let res = Response { status: Status::Ok, message: Some(Res::StringResult(v)) };
            serde_json::to_string(&res)
          },
          None => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(EMPTY_KEY)) }
          ),
        }
      },
      Command::Set(k, v) => {
        store.set(k, v);
        serde_json::to_string(
          &Response { status: Status::Ok, message: None }
        )
      },
      Command::Expire(k, at) => {
        store.expire(k, at);
        serde_json::to_string(
          &Response { status: Status::Ok, message: None }
        )
      },
      Command::Drop(k) => {
        store.drop(k);
        serde_json::to_string(
          &Response { status: Status::Ok, message: None }
        )
      },
      Command::LGet(k, i) => {
        match store.l_get(k, &i) {
          Some(ref mut v) => {
            let res = Response { status: Status::Ok, message: Some(Res::StringResult(v)) };
            serde_json::to_string(&res)
          },
          None => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(EMPTY_KEY)) }
          ),
        }
      },
      Command::LGetAll(k) => {
        match store.l_getall(k) {
          Some(ref mut v) => {
            let res = Response { status: Status::Ok, message: Some(Res::ListResult(v)) };
            serde_json::to_string(&res)
          },
          None => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(EMPTY_KEY)) }
          ),
        }
      },
      Command::LInsert(k, i, v) => {
        match store.l_insert(k, &i, v) {
          Ok(_) => {
            let res = Response { status: Status::Ok, message: None };
            serde_json::to_string(&res)
          },
          Err(m) => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(m)) }
          ),
        }
      },
      Command::LDrop(k, i) => {
        match store.l_drop(k, &i) {
          Ok(_) => {
            let res = Response { status: Status::Ok, message: None };
            serde_json::to_string(&res)
          },
          Err(m) => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(m)) }
          ),
        }
      },
      Command::HSet(key, subkey, v) => {
        match store.h_set(key, subkey, v) {
          Ok(_) => {
            let res = Response { status: Status::Ok, message: None };
            serde_json::to_string(&res)
          },
          Err(m) => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(m)) }
          ),
        }
      },
      Command::HGet(key, subkey) => {
        match store.h_get(key, subkey) {
          Some(ref mut v) => {
            let res = Response { status: Status::Ok, message: Some(Res::StringResult(v)) };
            serde_json::to_string(&res)
          },
          None => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(EMPTY_KEY)) }
          ),
        }
      },
      Command::HGetAll(key) => {
        match store.h_getall(key) {
          Some(ref mut v) => {
            let res = Response { status: Status::Ok, message: Some(Res::HashResult(v)) };
            serde_json::to_string(&res)
          },
          None => serde_json::to_string(
            &Response { status: Status::Error, message: Some(Res::RawResult(EMPTY_KEY)) }
          ),
        }
      },
      _ => serde_json::to_string(EMPTY_KEY),
    }
  }
}

#[derive(Debug)]
enum ParseError {
  UnexpectedCharacter(usize),
  IndexFormatError,
  InvalidCommand
}

impl From<ParseIntError> for ParseError {
  fn from(error: ParseIntError) -> Self {
    ParseError::IndexFormatError
  }
}

fn parse_argument(arguments_bytes: &mut [u8], skip_bytes: &mut Vec<usize>, start_position: usize) -> Result<(String, usize), ParseError> {
  let mut skip_white_spaces = true;
  let mut skip_next_quote = false;
  let mut index_of_ending_argument: usize = 0;
  let mut ending_byte: u8 = b' ';
  let mut index_of_beginning_argument: usize = 0;
  let mut skip_bytes: Vec<usize> = Vec::new();
  for (index, byte) in arguments_bytes.iter().enumerate() {
    index_of_ending_argument += 1;
    if *byte == b' ' && skip_white_spaces {
      index_of_beginning_argument += 1;
      continue
    }

    if skip_white_spaces {
      skip_white_spaces = false;
    }

    if ending_byte == b'"' && *byte == b'\\' {
      skip_next_quote = true;
      continue
    }

    if *byte != b'"' && skip_next_quote {
      skip_next_quote = false;
    }

    if *byte == b'"' && ending_byte == b'"' || *byte == 13 && (ending_byte == 13 || ending_byte == b' ') || *byte == b' ' && ending_byte == b' ' {
      if skip_next_quote {
        skip_next_quote = false;
        skip_bytes.push(index - 1);
        continue
      } else {
        break
      }
    }

    if *byte == 13 && ending_byte == b'"' {
      return Err(ParseError::UnexpectedCharacter(index + start_position))
    }

    if *byte == b'"' && ending_byte != b'"' {
      index_of_beginning_argument += 1;
      ending_byte = b'"'
    }
  }

  let mut argument = String::from_utf8_lossy(
    &mut arguments_bytes[index_of_beginning_argument..index_of_ending_argument-1]
  ).into_owned();

  let next_position = start_position + argument.len();

  for (index, x) in skip_bytes.iter().enumerate() {
    argument.remove(*x - index - index_of_beginning_argument);
  }

  return Ok((argument, next_position))
}

fn parse_query(query: &mut Vec<u8>, skip_bytes: &mut Vec<usize>) -> Result<Command, ParseError> {
  let index = query.iter().position(|&r| r == b' ');
  match index {
    Some(u) => {
      let (command_bytes, arguments_bytes) = query.split_at_mut(u);
      let command = String::from_utf8_lossy(command_bytes).into_owned();
      match command.as_ref() {
        "get" => {
          let (argument, _) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          return Ok(Command::Get(argument))
        },
        "set" => {
          let (key, next) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          let (value, _) = parse_argument(arguments_bytes, skip_bytes, next)?;
          return Ok(Command::Set(key, value))
        },
        "drop" => {
          let (argument, _) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          return Ok(Command::Drop(argument))
        },
        "expire" => {
          let (key, next) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          let (index_string, _) = parse_argument(arguments_bytes, skip_bytes, next)?;
          let t = index_string.parse::<u64>()?;
          return Ok(Command::Expire(key, Instant::now() + Duration::new(t, 0)))
        },
        "lget" => {
          let (key, next) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          let (index_string, _) = parse_argument(arguments_bytes, skip_bytes, next)?;
          let index = index_string.parse::<usize>()?;
          return Ok(Command::LGet(key, index))
        },
        "lgetall" => {
          let (argument, _) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          return Ok(Command::LGetAll(argument))
        },
        "linsert" => {
          let (key, next) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          let (index_string, next) = parse_argument(arguments_bytes, skip_bytes, next)?;
          let index = index_string.parse::<usize>()?;
          let (value, _) = parse_argument(arguments_bytes, skip_bytes, next)?;
          return Ok(Command::LInsert(key, index, value))
        },
        "ldrop" => {
          let (key, next) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          let (index_string, _) = parse_argument(arguments_bytes, skip_bytes, next)?;
          let index = index_string.parse::<usize>()?;
          return Ok(Command::LDrop(key, index))
        },
        "hget" => {
          let (key, next) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          let (subkey, _) = parse_argument(arguments_bytes, skip_bytes, next)?;
          return Ok(Command::HGet(key, subkey))
        },
        "hset" => {
          let (key, next) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          let (subkey, next) = parse_argument(arguments_bytes, skip_bytes, next)?;
          let (value, next) = parse_argument(arguments_bytes, skip_bytes, next)?;
          return Ok(Command::HSet(key, subkey, value))
        },
        "hgetall" => {
          let (key, _) = parse_argument(arguments_bytes, skip_bytes, command_bytes.len() + 1)?;
          return Ok(Command::HGetAll(key))
        },
        _ => return Err(ParseError::InvalidCommand)
      }
    },
    None => println!("nothing!")
  }
  Ok(Command::Get("123".to_string()))
}

fn read_to_block<R: Read + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize, Error> {
  let start_len = buf.len();
  let mut g = Guard { len: buf.len(), buf };
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
  store: Rc<RefCell<Store>>,
  addr: &'static str,
}

impl Server {
  pub fn new(addr: &'static str) -> Server {
    Server {
      store: Rc::new(RefCell::new(Store::new())),
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
        skip_bytes: Vec::with_capacity(100),
        store: Rc::clone(&mut self.store),
      };

      let handle_conn = h.map(|v| {
        println!("{}", v)
      }).map_err(|err| {
        println!("{:?}", err)
      });

      // Spawn the future as a concurrent task
      handler.spawn(handle_conn);

      Ok(())
    });

//    let shared_map: Rc<RefCell<_>> = Rc::new(RefCell::new(Store::new()));
//    let a = shared_map.borrow_mut();

    // Spin up the server on the event loop
    core.run(server).unwrap();
  }
}
