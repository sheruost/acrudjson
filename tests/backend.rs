//! A `Backend` over a plain `BTreeMap`, using only `core` and `alloc`.
//! This is the shape a `no_std` user would write for their own storage.

use acrudjson::op::Add;
use acrudjson::{Backend, BigDecimal, Error, Op, Operand};

use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
struct MapBackend(RefCell<BTreeMap<(String, String), BigDecimal>>);

impl MapBackend {
    fn get(&self, ns: &str, key: &str) -> Result<BigDecimal, Error> {
        let map = self.0.borrow();
        map.get(&(ns.into(), key.into()))
            .cloned()
            .ok_or(Error::KeyNotFound)
    }
}

impl Backend for MapBackend {
    fn create(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<(), Error> {
        let mut map = self.0.borrow_mut();
        let slot = map.entry((ns.into(), key.into()));
        match slot {
            std::collections::btree_map::Entry::Occupied(_) => Err(Error::KeyExists),
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(value.clone());
                Ok(())
            }
        }
    }

    fn read(&self, ns: &str, key: &str) -> Result<BigDecimal, Error> {
        self.get(ns, key)
    }

    fn update(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<BigDecimal, Error> {
        let mut map = self.0.borrow_mut();
        let slot = map
            .get_mut(&(ns.into(), key.into()))
            .ok_or(Error::KeyNotFound)?;
        Ok(std::mem::replace(slot, value.clone()))
    }

    fn delete(&self, ns: &str, key: &str) -> Result<BigDecimal, Error> {
        let mut map = self.0.borrow_mut();
        map.remove(&(ns.into(), key.into()))
            .ok_or(Error::KeyNotFound)
    }

    fn compute<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
        &self,
        ns: &str,
        _op: O,
        lhs: &L,
        rhs: &R,
    ) -> Result<BigDecimal, Error> {
        let l = lhs.resolve(|key| self.get(ns, key))?;
        let r = rhs.resolve(|key| self.get(ns, key))?;
        O::apply(&l, &r)
    }

    fn compute_into<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
        &self,
        ns: &str,
        op: O,
        lhs: &L,
        rhs: &R,
        into: &str,
    ) -> Result<BigDecimal, Error> {
        // Single-threaded, so read-compute-write is already atomic.
        let result = self.compute(ns, op, lhs, rhs)?;
        let mut map = self.0.borrow_mut();
        map.insert((ns.into(), into.into()), result.clone());
        Ok(result)
    }
}

#[test]
fn custom_backend_speaks_json_rpc() {
    let backend = MapBackend::default();
    let mut out = Vec::new();
    backend.handle(
        "t",
        br#"{"jsonrpc":"2.0","method":"create","params":["a",1.5],"id":1}"#,
        &mut out,
    );
    out.clear();
    backend.handle(
        "t",
        br#"{"jsonrpc":"2.0","method":"add","params":["a",2,"b"],"id":2}"#,
        &mut out,
    );
    assert_eq!(out, br#"{"jsonrpc":"2.0","result":3.5,"id":2}"#);
    assert_eq!(
        backend.read("t", "b").unwrap(),
        "3.5".parse::<BigDecimal>().unwrap()
    );
    assert_eq!(
        backend.compute("t", Add, "a", "b").unwrap(),
        "5".parse::<BigDecimal>().unwrap()
    );
}
