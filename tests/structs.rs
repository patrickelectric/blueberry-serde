use blueberry_serde::{deserialize, serialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Point {
    x: u32,
    y: u32,
}

#[test]
fn simple_struct_roundtrip() {
    let val = Point { x: 100, y: 200 };
    let bytes = serialize(&val).unwrap();
    assert_eq!(
        bytes,
        vec![
            0x64, 0x00, 0x00, 0x00, // x = 100
            0xC8, 0x00, 0x00, 0x00, // y = 200
        ]
    );
    let decoded: Point = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Nested {
    header: u32,
    point: Point,
    trailer: u8,
}

#[test]
fn nested_struct_inline() {
    let val = Nested {
        header: 0xAA,
        point: Point { x: 1, y: 2 },
        trailer: 0xFF,
    };
    let bytes = serialize(&val).unwrap();
    // header: [AA,00,00,00]
    // point.x: [01,00,00,00]
    // point.y: [02,00,00,00]
    // trailer: [FF]
    assert_eq!(
        bytes,
        vec![
            0xAA, 0x00, 0x00, 0x00, //
            0x01, 0x00, 0x00, 0x00, //
            0x02, 0x00, 0x00, 0x00, //
            0xFF,
        ]
    );
    let decoded: Nested = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MixedTypes {
    a: u8,
    b: u16,
    c: u32,
    d: u8,
}

#[test]
fn mixed_type_struct() {
    let val = MixedTypes {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
    };
    let bytes = serialize(&val).unwrap();
    // a: [01], pad: [00], b: [02,00], c: [03,00,00,00], d: [04]
    assert_eq!(
        bytes,
        vec![0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04]
    );
    let decoded: MixedTypes = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct AllU8 {
    a: u8,
    b: u8,
    c: u8,
}

#[test]
fn consecutive_u8_no_padding() {
    let val = AllU8 {
        a: 0x10,
        b: 0x20,
        c: 0x30,
    };
    let bytes = serialize(&val).unwrap();
    assert_eq!(bytes, vec![0x10, 0x20, 0x30]);
    let decoded: AllU8 = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn enum_unit_variants() {
    let v = Color::Blue;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![0x02, 0x00, 0x00, 0x00]); // variant index 2 as u32
    let decoded: Color = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NewtypeWrapper(u32);

#[test]
fn newtype_struct() {
    let val = NewtypeWrapper(42);
    let bytes = serialize(&val).unwrap();
    assert_eq!(bytes, vec![0x2A, 0x00, 0x00, 0x00]);
    let decoded: NewtypeWrapper = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}
