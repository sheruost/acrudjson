//! What a project depending on acrudjson reaches for first.

use acrudjson::client::{self, Read, Response};
use acrudjson::op::{Add, Multiply};
use acrudjson::{Backend, BigDecimal, Error, Store, V2};

use std::sync::Arc;

fn dec(s: &str) -> BigDecimal {
    s.parse().unwrap()
}

/// Generic code over any backend.
fn total(backend: &impl Backend) -> BigDecimal {
    backend.compute("t", Add, "a", "b").unwrap()
}

#[test]
fn shared_store_is_a_backend() {
    let store = Arc::new(Store::in_memory().unwrap());
    store.create("t", "a", &dec("1")).unwrap();
    store.create("t", "b", &dec("2")).unwrap();
    assert_eq!(total(&store), dec("3"), "Arc<Store>");
    assert_eq!(total(&&*store), dec("3"), "&Store");
    assert_eq!(total(&Box::new(store.clone())), dec("3"), "Box<Arc<Store>>");
}

#[test]
fn strings_and_integers_are_operands() {
    let store = Store::in_memory().unwrap();
    let key = String::from("x");
    store.create("t", &key, &dec("1.5")).unwrap();
    assert_eq!(store.compute("t", Multiply, &key, &2).unwrap(), dec("3"));
    assert_eq!(
        store.compute("t", Add, &-1_i64, &u128::MAX).unwrap(),
        dec("340282366920938463463374607431768211454")
    );
}

#[test]
fn rpc_errors_can_be_boxed() -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::in_memory()?;
    let (mut request, mut reply) = (Vec::new(), Vec::new());
    client::request::<V2, _>(&mut request, 1, &Read { key: "missing" });
    store.handle("t", &request, &mut reply);
    let response = Response::try_from(&reply[..])?;
    let err = response
        .result()
        .map_err(|e| e.clone().into_owned())
        .unwrap_err();
    assert_eq!(err.code, acrudjson::code::KEY_NOT_FOUND as i64);
    let boxed: Box<dyn std::error::Error> = Box::new(err);
    assert_eq!(boxed.to_string(), "Key not found (-32001)");
    Ok(())
}

#[test]
fn errors_convert_to_io() {
    let e: std::io::Error = Error::KeyNotFound.into();
    assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn common_traits() {
    let store = Store::in_memory().unwrap();
    assert!(format!("{store:?}").starts_with("Store"));
    assert_eq!(Add.to_string(), "add");
    assert_eq!(V2, V2);
}
