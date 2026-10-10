//! Wire-level behaviour, checked against the examples in the JSON-RPC 2.0
//! specification (https://www.jsonrpc.org/specification#examples) plus 1.0.

use acrudjson::{Backend, Store};

use serde_json::{Value, json};

fn store() -> Store {
    let store = Store::in_memory().unwrap();
    for (key, value) in [("a", "6"), ("b", "4")] {
        let mut out = Vec::new();
        let req =
            format!(r#"{{"jsonrpc":"2.0","method":"create","params":["{key}",{value}],"id":0}}"#);
        store.handle("t", req.as_bytes(), &mut out);
    }
    store
}

/// Sends `request` and returns the parsed response, or `None` when nothing was written.
fn call(store: &Store, request: &str) -> Option<Value> {
    let mut out = Vec::new();
    let wrote = store.handle("t", request.as_bytes(), &mut out);
    assert_eq!(wrote, !out.is_empty());
    wrote.then(|| serde_json::from_slice(&out).expect("response is valid JSON"))
}

#[test]
fn positional_and_named_params() {
    let s = store();
    assert_eq!(
        call(
            &s,
            r#"{"jsonrpc":"2.0","method":"subtract","params":["a","b"],"id":1}"#
        ),
        Some(json!({"jsonrpc":"2.0","result":2,"id":1}))
    );
    assert_eq!(
        call(
            &s,
            r#"{"jsonrpc":"2.0","method":"subtract","params":{"rhs":"a","lhs":"b"},"id":"x"}"#
        ),
        Some(json!({"jsonrpc":"2.0","result":-2,"id":"x"}))
    );
}

#[test]
fn notifications_get_no_response() {
    let s = store();
    assert_eq!(
        call(
            &s,
            r#"{"jsonrpc":"2.0","method":"update","params":["a",1]}"#
        ),
        None
    );
    assert_eq!(
        call(&s, r#"{"jsonrpc":"2.0","method":"nope"}"#),
        None,
        "even when they fail"
    );
    assert_eq!(
        call(&s, r#"{"method":"read","params":["a"],"id":null}"#),
        None,
        "1.0 notification"
    );
    // `"id": null` is a request in 2.0, not a notification.
    assert_eq!(
        call(
            &s,
            r#"{"jsonrpc":"2.0","method":"read","params":["a"],"id":null}"#
        ),
        Some(json!({"jsonrpc":"2.0","result":1,"id":null}))
    );
}

#[test]
fn standard_errors() {
    let s = store();
    let error = |r: Option<Value>| {
        (
            r.as_ref().unwrap()["error"]["code"].as_i64().unwrap(),
            r.unwrap()["id"].clone(),
        )
    };
    assert_eq!(
        error(call(&s, r#"{"jsonrpc":"2.0","method":"foobar","id":"1"}"#)),
        (-32601, json!("1"))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"foobar,"params":"bar","baz]"#
        )),
        (-32700, Value::Null)
    );
    assert_eq!(
        error(call(&s, r#"{"jsonrpc":"2.0","method":1,"params":"bar"}"#)),
        (-32600, Value::Null)
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"read","params":[1],"id":2}"#
        )),
        (-32602, json!(2))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"read","params":["a","b"],"id":3}"#
        )),
        (-32602, json!(3))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"read","params":{"key":"a","x":1},"id":4}"#
        )),
        (-32602, json!(4))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"read","params":["zz"],"id":5}"#
        )),
        (-32001, json!(5))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"create","params":["a",1],"id":6}"#
        )),
        (-32002, json!(6))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"divide","params":["a",0],"id":7}"#
        )),
        (-32003, json!(7))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"add","params":["a",1e-1000000],"id":8}"#
        )),
        (-32004, json!(8))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"3.0","method":"read","params":["a"],"id":9}"#
        )),
        (-32600, json!(9))
    );
    assert_eq!(
        error(call(
            &s,
            r#"{"jsonrpc":"2.0","method":"read","params":["a"],"id":{}}"#
        )),
        (-32600, Value::Null)
    );
}

#[test]
fn batches() {
    let s = store();
    // The spec's mixed batch: requests, a notification, junk, and an unknown method.
    let response = call(
        &s,
        r#"[
            {"jsonrpc":"2.0","method":"add","params":["a","b"],"id":"1"},
            {"jsonrpc":"2.0","method":"update","params":["b",7]},
            {"jsonrpc":"2.0","method":"subtract","params":["a","b"],"id":"2"},
            {"foo":"boo"},
            {"jsonrpc":"2.0","method":"foo.get","params":{"name":"myself"},"id":"5"},
            1
        ]"#,
    )
    .unwrap();
    let codes: Vec<_> = response
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["result"].clone(),
                r["error"]["code"].clone(),
                r["id"].clone(),
            )
        })
        .collect();
    assert_eq!(
        codes,
        [
            (json!(10), Value::Null, json!("1")),
            // Requests run in order: the notification above already set b = 7.
            (json!(-1), Value::Null, json!("2")),
            (Value::Null, json!(-32600), Value::Null),
            (Value::Null, json!(-32601), json!("5")),
            (Value::Null, json!(-32600), Value::Null),
        ]
    );

    assert_eq!(
        call(&s, "[]").unwrap()["error"]["code"],
        -32600,
        "empty batch is a single error"
    );
    assert_eq!(
        call(&s, r#"[{"jsonrpc":"2.0","method":"read","params":["a"]}]"#),
        None,
        "all notifications"
    );
    assert_eq!(
        call(
            &s,
            r#"[{"jsonrpc":"2.0","method":"delete","params":["a"],"id":1}, {"jsonrpc""#
        )
        .unwrap()["error"]["code"],
        -32700
    );
    assert_eq!(
        call(
            &s,
            r#"{"jsonrpc":"2.0","method":"read","params":["a"],"id":1}"#
        )
        .unwrap()["result"],
        6,
        "invalid batch ran nothing"
    );
}

#[test]
fn json_rpc_1_0() {
    let s = store();
    assert_eq!(
        call(&s, r#"{"method":"multiply","params":["a","b"],"id":1}"#),
        Some(json!({"result":24,"error":null,"id":1}))
    );
    // 0.1.x clients sent every param as a string; values in strings still work.
    assert_eq!(
        call(
            &s,
            r#"{"jsonrpc":"1.0","method":"create","params":["c","0.5"],"id":2}"#
        )
        .unwrap()["error"]["code"],
        -32600,
        "\"jsonrpc\":\"1.0\" was never part of the 1.0 spec"
    );
    assert_eq!(
        call(&s, r#"{"method":"create","params":["c","0.5"],"id":2}"#),
        Some(json!({"result":true,"error":null,"id":2}))
    );
    let error = call(&s, r#"{"method":"read","params":["nope"],"id":3}"#).unwrap();
    assert_eq!(
        (error["result"].clone(), error["error"]["code"].clone()),
        (Value::Null, json!(-32001))
    );
}

#[test]
fn numbers_keep_every_digit() {
    let s = store();
    let big = "6416930923733925522307001.29472615";
    let g = "0.000000000066731039356729";
    let mut out = Vec::new();
    s.handle(
        "t",
        format!(
            r#"{{"jsonrpc":"2.0","method":"create","params":{{"key":"M","value":{big}}},"id":1}}"#
        )
        .as_bytes(),
        &mut out,
    );
    out.clear();
    s.handle(
        "t",
        format!(r#"{{"jsonrpc":"2.0","method":"multiply","params":["M",{g},"GM"],"id":2}}"#)
            .as_bytes(),
        &mut out,
    );
    let response = acrudjson::client::Response::try_from(&out[..]).unwrap();
    let expected: acrudjson::BigDecimal = "428208470021099.96114484339101847547483621476335"
        .parse()
        .unwrap();
    assert_eq!(response.number().unwrap(), Some(expected.clone()));
    assert_eq!(s.read("t", "GM").unwrap(), expected);
    // Every format `Display` produces must be JSON number syntax. Results
    // beyond f64 range are still valid JSON (RFC 8259 sets no limit), though
    // `serde_json::Value` refuses them; the raw-value client reads them exactly.
    for n in ["1e30", "-1e-30", "0", "123.4500", "1E+999999", "-7e-999999"] {
        let mut out = Vec::new();
        s.handle(
            "t",
            format!(r#"{{"jsonrpc":"2.0","method":"add","params":[{n},0],"id":1}}"#).as_bytes(),
            &mut out,
        );
        serde_json::from_slice::<serde::de::IgnoredAny>(&out)
            .unwrap_or_else(|e| panic!("{n}: {e}"));
        let response = acrudjson::client::Response::try_from(&out[..]).unwrap();
        assert_eq!(response.number().unwrap(), Some(n.parse().unwrap()), "{n}");
    }
    // Validating a batch must not trip over numbers beyond f64 either.
    let batch = call(
        &s,
        r#"[{"jsonrpc":"2.0","method":"multiply","params":[1e400,0],"id":1}]"#,
    )
    .unwrap();
    assert_eq!(batch[0]["result"].as_f64(), Some(0.0));
}

#[test]
fn escaped_keys_and_reused_buffers() {
    let s = store();
    let mut out = b"prefix:".to_vec();
    assert!(s.handle(
        "t",
        r#"{"jsonrpc":"2.0","method":"create","params":["kéy\n",1],"id":1}"#.as_bytes(),
        &mut out
    ));
    assert!(out.starts_with(b"prefix:{"), "handle appends, never clears");
    assert_eq!(
        s.read("t", "k\u{e9}y\n").unwrap(),
        acrudjson::BigDecimal::from(1)
    );
}

#[test]
fn invalid_params_list_the_expected_names() {
    let s = store();
    let r = call(
        &s,
        r#"{"jsonrpc":"2.0","method":"create","params":{"nope":1},"id":1}"#,
    )
    .unwrap();
    assert_eq!(r["error"]["data"], json!(["key", "value"]));
    let r = call(&s, r#"{"jsonrpc":"2.0","method":"add","params":[],"id":2}"#).unwrap();
    assert_eq!(r["error"]["data"], json!(["lhs", "rhs", "into"]));
}

#[test]
fn batch_elements_must_be_objects() {
    let s = store();
    // A derived struct would also accept an array, filled by position.
    let r = call(&s, r#"[["2.0","update",["a",1],7]]"#).unwrap();
    assert_eq!(r[0]["error"]["code"], -32600);
    assert_eq!(r[0]["id"], Value::Null);
    let r = call(
        &s,
        r#"{"jsonrpc":"2.0","method":"read","params":["a"],"id":1}"#,
    )
    .unwrap();
    assert_eq!(r["result"], 6, "the array element must not run");
}
