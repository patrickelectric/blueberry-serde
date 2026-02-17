use blueberry_serde::{deserialize, serialize};

#[test]
fn u8_roundtrip() {
    let v: u8 = 0xAB;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![0xAB]);
    let decoded: u8 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn i8_roundtrip() {
    let v: i8 = -42;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![(-42i8) as u8]);
    let decoded: i8 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn u16_little_endian() {
    let v: u16 = 0xABCD;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![0xCD, 0xAB]);
    let decoded: u16 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn i16_little_endian() {
    let v: i16 = -32700;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![0x44, 0x80]);
    let decoded: i16 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn u32_little_endian() {
    let v: u32 = 0xDEADBEEF;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![0xEF, 0xBE, 0xAD, 0xDE]);
    let decoded: u32 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn i32_little_endian() {
    let v: i32 = -2_147_483_600;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![0x30, 0x00, 0x00, 0x80]);
    let decoded: i32 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn u64_little_endian() {
    let v: u64 = 0x0102030405060708;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes, vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]);
    let decoded: u64 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn i64_little_endian() {
    let v: i64 = -9_223_372_036_800;
    let bytes = serialize(&v).unwrap();
    let decoded: i64 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn f32_little_endian() {
    let v: f32 = std::f32::consts::PI;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes.len(), 4);
    let decoded: f32 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}

#[test]
fn f64_little_endian() {
    let v: f64 = std::f64::consts::PI;
    let bytes = serialize(&v).unwrap();
    assert_eq!(bytes.len(), 8);
    let decoded: f64 = deserialize(&bytes).unwrap();
    assert_eq!(v, decoded);
}
