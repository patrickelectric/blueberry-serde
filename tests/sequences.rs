use blueberry_serde::{deserialize, serialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WithSequence {
    id: u32,
    items: Vec<u32>,
}

#[test]
fn sequence_with_u32_elements() {
    let val = WithSequence {
        id: 1,
        items: vec![10, 20, 30],
    };
    let bytes = serialize(&val).unwrap();

    // Body layout:
    //   u32 id = [01,00,00,00]
    //   seq header: u16 index (to be fixed up), u16 elementByteLength=4
    // Data block (appended):
    //   u32 count = 3
    //   u32 10, u32 20, u32 30

    let decoded: WithSequence = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn empty_sequence() {
    let val = WithSequence {
        id: 42,
        items: vec![],
    };
    let bytes = serialize(&val).unwrap();
    let decoded: WithSequence = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WithU8Sequence {
    tag: u8,
    data: Vec<u8>,
}

#[test]
fn sequence_of_u8() {
    let val = WithU8Sequence {
        tag: 0xFF,
        data: vec![1, 2, 3, 4, 5],
    };
    let bytes = serialize(&val).unwrap();
    let decoded: WithU8Sequence = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TwoSequences {
    a: Vec<u16>,
    b: Vec<u32>,
}

#[test]
fn multiple_sequences() {
    let val = TwoSequences {
        a: vec![1, 2, 3],
        b: vec![100, 200],
    };
    let bytes = serialize(&val).unwrap();
    let decoded: TwoSequences = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Inner {
    x: u16,
    y: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WithStructSequence {
    count: u32,
    points: Vec<Inner>,
}

#[test]
fn sequence_of_structs_packed_without_padding() {
    let val = WithStructSequence {
        count: 2,
        points: vec![Inner { x: 1, y: 2 }, Inner { x: 3, y: 4 }],
    };
    let bytes = serialize(&val).unwrap();
    let decoded: WithStructSequence = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}
