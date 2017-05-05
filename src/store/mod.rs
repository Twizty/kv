use std::collections::HashMap;
use std::sync::{Mutex};
use std::borrow::Borrow;

enum V {
  StringValue(String),
  ListValue(Vec<String>),
  MapValue(HashMap<String, String>),
}

pub struct Store {
  mutex: Mutex<()>,
  store: HashMap<String, V>,
}

impl Store {
  pub fn new() -> Store {
    Store { mutex: Mutex::new(()), store: HashMap::new() }
  }

  pub fn get(&self, key: String) -> Option<&String> {
    let _data = self.mutex.lock().unwrap();

    match self.store.get(&key) {
      Some(value) => {
        match value {
          &V::StringValue(ref s) => Some(s),
          _ => None,
        }
      },
      None => None,
    }
  }

  pub fn set(&mut self, key: String, val: String) {
    let _data = self.mutex.lock().unwrap();

    self.store.insert(key, V::StringValue(val));
  }
}


mod test {
  use super::*;

  #[test]
  fn test_set_and_get() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";
    s.set(key.clone().to_string(), value.clone().to_string());

    assert_eq!(s.get(key.clone().to_string()), Some(&(value.to_string())));
    assert_eq!(s.get("baz".to_string()), None);
  }
}
