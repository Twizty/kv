use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Mutex};
use std::time::{Instant};

const TYPE_MISSMATCH_ERROR: &'static str = "Type missmatch for the key";
const KEY_NOT_FOUND_ERROR: &'static str = "Key not found";
const INDEX_OUT_OF_RANGE_ERROR: &'static str = "Index is out of range";

type StringValue = String;
type ListValue = Vec<String>;
type HashValue = HashMap<String, String>;

enum V {
  StringValue(StringValue),
  ListValue(ListValue),
  HashValue(HashValue),
}

pub struct Store {
  mutex: Mutex<()>,
  store: HashMap<String, V>,
  ttls:  HashMap<String, Instant>,
}

fn drop_if_expired(e: Entry<String, Instant>, store: &mut HashMap<String, V>) -> Option<()> {
  let now = Instant::now();

  if let Entry::Occupied(occ_e) = e {
    if now >= *occ_e.get() {
      store.remove(occ_e.key());
      occ_e.remove();
      return Some(())
    }
  }

  None
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

    if let Some(_) = drop_if_expired(self.ttls.entry(key.clone()), &mut self.store) {
      return None
    }

    if let Some(value) = self.store.get(&key) {
      match value {
        &V::StringValue(ref s) => {
          Some(s)
        },
        _ => None,
      }
    } else {
      None
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

  pub fn l_append(&mut self, key: String, val: String) -> Result<(), &'static str> {
    let _data = self.mutex.lock().unwrap();

    drop_if_expired(self.ttls.entry(key.clone()), &mut self.store);

    let mut value = self.store.entry(key.clone()).or_insert(V::ListValue(Vec::new()));

    if let &mut V::ListValue(ref mut s) = value {
      s.push(val);
      Ok(())
    } else {
      Err(TYPE_MISSMATCH_ERROR)
    }
  }

  pub fn l_get(&mut self, key: String, index: &usize) -> Option<&String> {
    let _data = self.mutex.lock().unwrap();

    if let Some(_) = drop_if_expired(self.ttls.entry(key.clone()), &mut self.store) {
      return None
    }

    match self.store.get(&key) {
      Some(value) => {
        match value {
          &V::ListValue(ref v) if v.len() > *index => Some(&v[*index]),
          _ => None,
        }
      },
      None => None,
    }
  }

  pub fn l_getall(&mut self, key: String) -> Option<&Vec<String>> {
    let _data = self.mutex.lock().unwrap();

    if let Some(_) = drop_if_expired(self.ttls.entry(key.clone()), &mut self.store) {
      return None
    }

    match self.store.get(&key) {
      Some(value) => {
        match value {
          &V::ListValue(ref v) => Some(v),
          _ => None,
        }
      },
      None => None,
    }
  }

  pub fn l_insert(&mut self, key: String, index: &usize, val: String) -> Result<(), &'static str> {
    let _data = self.mutex.lock().unwrap();

    if let Some(_) = drop_if_expired(self.ttls.entry(key.clone()), &mut self.store) {
      return Err(KEY_NOT_FOUND_ERROR)
    }

    match self.store.get_mut(&key) {
      Some(value) => {
        match value {
          &mut V::ListValue(ref mut list) => {
            if list.len() > *index {
              list.insert(*index, val);
              Ok(())
            } else {
              Err(INDEX_OUT_OF_RANGE_ERROR)
            }
          },
          _ => Err(TYPE_MISSMATCH_ERROR)
        }
      },
      None => Err(KEY_NOT_FOUND_ERROR)
    }
  }

  pub fn drop(&mut self, key: String) {
    let _data = self.mutex.lock().unwrap();

    self.store.remove(&key);
    self.ttls.remove(&key);
  }

  pub fn l_drop(&mut self, key: String, index: &usize) -> Result<(), &'static str> {
    let _data = self.mutex.lock().unwrap();

    if let Some(_) = drop_if_expired(self.ttls.entry(key.clone()), &mut self.store) {
      return Err(KEY_NOT_FOUND_ERROR)
    }

    match self.store.get_mut(&key) {
      Some(value) => {
        match value {
          &mut V::ListValue(ref mut list) => {
            if list.len() > *index {
              list.remove(*index);
              Ok(())
            } else {
              Err(INDEX_OUT_OF_RANGE_ERROR)
            }
          },
          _ => Err(TYPE_MISSMATCH_ERROR)
        }
      },
      None => Err(KEY_NOT_FOUND_ERROR)
    }
  }

  pub fn h_set(&mut self, key: String, h_key: String, val: String) -> Result<(), &'static str> {
    let _data = self.mutex.lock().unwrap();

    drop_if_expired(self.ttls.entry(key.clone()), &mut self.store);

    let mut value = self.store.entry(key.clone()).or_insert(V::HashValue(HashMap::new()));

    if let &mut V::HashValue(ref mut map) = value {
      map.insert(h_key, val);
      Ok(())
    } else {
      Err(TYPE_MISSMATCH_ERROR)
    }
  }

  pub fn h_get(&mut self, key: String, h_key: String) -> Option<&String> {
    let _data = self.mutex.lock().unwrap();

    if let Some(_) = drop_if_expired(self.ttls.entry(key.clone()), &mut self.store) {
      return None
    }

    match self.store.get_mut(&key) {
      Some(value) => {
        if let &mut V::HashValue(ref mut map) = value {
          if let Some(ref v) = map.get(&h_key) {
            Some(v)
          } else {
            None
          }
        } else {
          None
        }
      },
      None => None
    }
  }

  pub fn h_getall(&mut self, key: String) -> Option<&HashMap<String, String>> {
    let _data = self.mutex.lock().unwrap();

    if let Some(_) = drop_if_expired(self.ttls.entry(key.clone()), &mut self.store) {
      return None
    }

    match self.store.get(&key) {
      Some(value) => {
        match value {
          &V::HashValue(ref v) => Some(v),
          _ => None,
        }
      },
      None => None,
    }
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
    s.set(key.to_string(), value.to_string());

    assert_eq!(s.get(key.to_string()), Some(&value.to_string()));
    assert_eq!(s.get("baz".to_string()), None);
  }

  #[test]
  fn test_get_after_expire() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";
    s.set(key.to_string(), value.to_string());
    s.expire(key.to_string(), Instant::now() + Duration::new(1, 0));
    assert_eq!(s.get(key.to_string()), Some(&value.to_string()));

    sleep(Duration::new(1, 0));

    assert_eq!(s.get(key.to_string()), None);
  }

  #[test]
  fn test_append_key() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";
    let result = s.l_append(key.to_string(), value.to_string());

    assert_eq!(result, Ok(()));
    assert_eq!(s.l_get(key.to_string(), &0), Some(&value.to_string()));
    assert_eq!(s.l_get(key.to_string(), &1), None);
  }

  #[test]
  fn test_returns_error_if_key_is_not_a_list() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";

    s.set(key.to_string(), value.to_string());
    let result = s.l_append(key.to_string(), value.to_string());

    assert_eq!(result, Err(TYPE_MISSMATCH_ERROR));
  }

  #[test]
  fn test_when_list_expired_new_one_created() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";
    let result = s.l_append(key.to_string(), value.to_string());

    s.expire(key.to_string(), Instant::now() + Duration::new(1, 0));
    assert_eq!(result, Ok(()));

    sleep(Duration::new(1, 0));

    let new_value = "baz";
    s.l_append(key.to_string(), new_value.to_string());

    assert_eq!(s.l_get(key.to_string(), &0), Some(&new_value.to_string()));
  }

  #[test]
  fn test_list_getall() {
    let mut s = Store::new();
    let key = "foo";
    let value1 = "bar";
    let value2 = "baz";

    s.l_append(key.to_string(), value1.to_string());
    s.l_append(key.to_string(), value2.to_string());

    let result = s.l_getall(key.to_string());

    assert_eq!(result, Some(&vec!["bar".to_string(), "baz".to_string()]));
  }

  #[test]
  fn test_list_insert() {
    let mut s = Store::new();
    let key = "foo";
    let value1 = "bar";
    let value2 = "baz";
    let value3 = "foobar";

    s.l_append(key.to_string(), value1.to_string());
    s.l_append(key.to_string(), value2.to_string());
    s.l_insert(key.to_string(), &0, value3.to_string());

    {
      let result = s.l_getall(key.to_string());
      assert_eq!(result, Some(&vec!["foobar".to_string(), "bar".to_string(), "baz".to_string()]));
    }

    {
      let new_result = s.l_insert(key.to_string(), &5, value3.to_string());
      assert_eq!(new_result, Err(INDEX_OUT_OF_RANGE_ERROR));
    }

    {
      s.set(key.to_string(), value1.to_string());
      let new_result = s.l_insert(key.to_string(), &0, value2.to_string());
      assert_eq!(new_result, Err(TYPE_MISSMATCH_ERROR));
    }

    {
      let new_key = "foobaz";
      let new_result = s.l_insert(new_key.to_string(), &0, value1.to_string());
      assert_eq!(new_result, Err(KEY_NOT_FOUND_ERROR));
    }
  }

  #[test]
  fn test_list_drop() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";

    s.l_append(key.to_string(), value.to_string());

    {
      s.l_drop(key.to_string(), &0);
      let result = s.l_getall(key.to_string());
      assert_eq!(result, Some(&vec![]));
    }
  }

  #[test]
  fn test_drop() {
    let mut s = Store::new();
    let key = "foo";
    let value = "bar";

    s.set(key.to_string(), value.to_string());

    assert_eq!(s.get(key.to_string()), Some(&value.to_string()));

    s.drop(key.to_string());

    assert_eq!(s.get(key.to_string()), None);
  }

  #[test]
  fn test_hash_getall() {
    let mut s = Store::new();
    let key = "foo";
    let h_key1 = "foobar";
    let h_key2 = "foobaz";
    let value1 = "bar";
    let value2 = "baz";

    s.h_set(key.to_string(), h_key1.to_string(), value1.to_string());

    {
      let result = s.h_get(key.to_string(), h_key1.to_string());

      assert_eq!(result, Some(&value1.to_string()));
    }

    s.h_set(key.to_string(), h_key2.to_string(), value2.to_string());

    {
      let new_result = s.h_getall(key.to_string()).unwrap();

      {assert_eq!(new_result.get(&h_key1.to_string()), Some(&value1.to_string()));}
      {assert_eq!(new_result.get(&h_key2.to_string()), Some(&value2.to_string()));}
    }
  }
}
