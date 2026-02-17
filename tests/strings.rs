use blueberry_serde::{deserialize, serialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WithString {
    id: u32,
    name: String,
}

#[test]
fn string_roundtrip() {
    let val = WithString {
        id: 42,
        name: "hello".to_string(),
    };
    let bytes = serialize(&val).unwrap();
    let decoded: WithString = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn empty_string() {
    let val = WithString {
        id: 1,
        name: String::new(),
    };
    let bytes = serialize(&val).unwrap();
    let decoded: WithString = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn utf8_string() {
    let val = WithString {
        id: 7,
        name: "héllo wörld".to_string(),
    };
    let bytes = serialize(&val).unwrap();
    let decoded: WithString = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TwoStrings {
    a: String,
    b: String,
}

#[test]
fn multiple_strings() {
    let val = TwoStrings {
        a: "foo".to_string(),
        b: "bar".to_string(),
    };
    let bytes = serialize(&val).unwrap();
    let decoded: TwoStrings = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}
