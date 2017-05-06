use std::collections::HashMap;
// use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};
use std::borrow::Borrow;
use std::time::{Instant};

enum V {
  StringValue(String),
  ListValue(Vec<String>),
  MapValue(HashMap<String, String>),
}

pub struct Store {
  mutex: Mutex<()>,
  store: HashMap<String, V>,
  ttls:  HashMap<String, Instant>,
}

impl Store {
  pub fn new() -> Store {
    Store {
      mutex: Mutex::new(()),
      store: HashMap::new(),
      ttls:  HashMap::new(),
    }
  }

  pub fn get(&mut self, key: String) -> Option<&String> {
    let _data = self.mutex.lock().unwrap();
    let now   = Instant::now();

    if let Entry::Occupied(occ_e) = self.ttls.entry(key.clone()) {
      if now >= *occ_e.get() {
        occ_e.remove();
        self.store.remove(&key);
        return None
      }
    }

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

  pub fn expire(&mut self, key: String, at: Instant) {
    let _data = self.mutex.lock().unwrap();

   self.ttls.insert(key, at); 
  }
}


mod test {
  use super::*;
  use std::thread::sleep;
  use std::time::{Duration};

  #[test]
  fn test_set_and_get() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";
    s.set(key.clone().to_string(), value.clone().to_string());

    assert_eq!(s.get(key.clone().to_string()), Some(&(value.to_string())));
    assert_eq!(s.get("baz".to_string()), None);
  }

  fn test_get_after_expire() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";
    s.set(key.clone().to_string(), value.clone().to_string());
    s.expire(key.clone().to_string(), Instant::now() + Duration::new(1, 0));
    assert_eq!(s.get(key.clone().to_string()), Some(&(value.to_string())));

    sleep(Duration::new(1, 0));

    assert_eq!(s.get(key.clone().to_string()), None);
  }
}
