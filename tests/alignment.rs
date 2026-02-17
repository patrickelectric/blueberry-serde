use blueberry_serde::{deserialize, serialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MixedAlign {
    a: u32, // offset 0, 4 bytes
    b: u8,  // offset 4, 1 byte
    // 1 byte padding at offset 5 to align u16
    c: u16, // offset 6, 2 bytes
}

#[test]
fn u16_after_u8_has_padding() {
    let val = MixedAlign { a: 1, b: 2, c: 3 };
    let bytes = serialize(&val).unwrap();
    // u32(1) = [01,00,00,00], u8(2) = [02], pad=[00], u16(3) = [03,00]
    assert_eq!(bytes, vec![0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x03, 0x00]);
    let decoded: MixedAlign = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct U64Alignment {
    a: u8, // offset 0
    // 3 bytes padding to align u64 on 4-byte boundary (NOT 8-byte)
    b: u64, // offset 4 (4-byte aligned)
}

#[test]
fn u64_aligns_on_4_byte_boundary() {
    let val = U64Alignment { a: 0xFF, b: 1 };
    let bytes = serialize(&val).unwrap();
    // u8(0xFF) = [FF], pad=[00,00,00], u64(1) = [01,00,00,00,00,00,00,00]
    assert_eq!(
        bytes,
        vec![0xFF, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
    let decoded: U64Alignment = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct F64Alignment {
    a: u16, // offset 0
    // 2 bytes padding to 4-byte boundary
    b: f64, // offset 4 (4-byte aligned, not 8)
}

#[test]
fn f64_aligns_on_4_byte_boundary() {
    let val = F64Alignment { a: 0x0102, b: 1.0 };
    let bytes = serialize(&val).unwrap();
    // u16 = [02,01], pad=[00,00], f64(1.0) in LE
    assert_eq!(bytes.len(), 12);
    assert_eq!(bytes[0..2], [0x02, 0x01]);
    assert_eq!(bytes[2..4], [0x00, 0x00]); // padding
    let decoded: F64Alignment = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct U32AfterU8 {
    a: u8, // offset 0
    // 3 bytes padding
    b: u32, // offset 4
}

#[test]
fn u32_after_u8_padding() {
    let val = U32AfterU8 { a: 0x42, b: 0x01 };
    let bytes = serialize(&val).unwrap();
    assert_eq!(bytes, vec![0x42, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
    let decoded: U32AfterU8 = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NoPadding {
    a: u32,
    b: u32,
    c: u32,
}

#[test]
fn naturally_aligned_no_padding() {
    let val = NoPadding { a: 1, b: 2, c: 3 };
    let bytes = serialize(&val).unwrap();
    assert_eq!(bytes.len(), 12);
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x00, 0x00, //
            0x02, 0x00, 0x00, 0x00, //
            0x03, 0x00, 0x00, 0x00,
        ]
    );
    let decoded: NoPadding = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}
