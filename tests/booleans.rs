use blueberry_serde::{deserialize, serialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TwoBools {
    a: bool, // bit 0
    b: bool, // bit 1
}

#[test]
fn two_bools_packed_into_one_byte() {
    let val = TwoBools { a: true, b: true };
    let bytes = serialize(&val).unwrap();
    // Both packed: bit0=1, bit1=1 => 0b11 = 3
    assert_eq!(bytes, vec![0x03]);
    let decoded: TwoBools = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn two_bools_first_true() {
    let val = TwoBools { a: true, b: false };
    let bytes = serialize(&val).unwrap();
    // bit0=1, bit1=0 => 0b01 = 1
    assert_eq!(bytes, vec![0x01]);
    let decoded: TwoBools = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn two_bools_second_true() {
    let val = TwoBools { a: false, b: true };
    let bytes = serialize(&val).unwrap();
    // bit0=0, bit1=1 => 0b10 = 2
    assert_eq!(bytes, vec![0x02]);
    let decoded: TwoBools = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn two_bools_both_false() {
    let val = TwoBools { a: false, b: false };
    let bytes = serialize(&val).unwrap();
    assert_eq!(bytes, vec![0x00]);
    let decoded: TwoBools = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct EightBools {
    b0: bool,
    b1: bool,
    b2: bool,
    b3: bool,
    b4: bool,
    b5: bool,
    b6: bool,
    b7: bool,
}

#[test]
fn eight_bools_all_true_one_byte() {
    let val = EightBools {
        b0: true,
        b1: true,
        b2: true,
        b3: true,
        b4: true,
        b5: true,
        b6: true,
        b7: true,
    };
    let bytes = serialize(&val).unwrap();
    assert_eq!(bytes, vec![0xFF]);
    let decoded: EightBools = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn eight_bools_alternating() {
    let val = EightBools {
        b0: true,
        b1: false,
        b2: true,
        b3: false,
        b4: true,
        b5: false,
        b6: true,
        b7: false,
    };
    let bytes = serialize(&val).unwrap();
    // bits: 0b01010101 = 0x55
    assert_eq!(bytes, vec![0x55]);
    let decoded: EightBools = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct BoolsWithU32 {
    flag1: bool,
    flag2: bool,
    // bool packing flushed before u32
    value: u32,
}

#[test]
fn bools_flushed_before_non_bool() {
    let val = BoolsWithU32 {
        flag1: true,
        flag2: false,
        value: 42,
    };
    let bytes = serialize(&val).unwrap();
    // bool byte: bit0=1, bit1=0 => 0x01
    // padding: 3 bytes to align u32
    // u32(42) = [2A, 00, 00, 00]
    assert_eq!(bytes, vec![0x01, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00]);
    let decoded: BoolsWithU32 = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct BoolAfterU32 {
    value: u32,
    flag: bool,
}

#[test]
fn single_bool_after_u32() {
    let val = BoolAfterU32 {
        value: 1,
        flag: true,
    };
    let bytes = serialize(&val).unwrap();
    // u32(1) = [01,00,00,00], bool(true) = [01]
    assert_eq!(bytes, vec![0x01, 0x00, 0x00, 0x00, 0x01]);
    let decoded: BoolAfterU32 = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NineBools {
    b0: bool,
    b1: bool,
    b2: bool,
    b3: bool,
    b4: bool,
    b5: bool,
    b6: bool,
    b7: bool,
    b8: bool, // overflows into second byte
}

#[test]
fn nine_bools_two_bytes() {
    let val = NineBools {
        b0: true,
        b1: true,
        b2: true,
        b3: true,
        b4: true,
        b5: true,
        b6: true,
        b7: true,
        b8: true,
    };
    let bytes = serialize(&val).unwrap();
    // First byte: all 8 bits set = 0xFF
    // Second byte: bit0 set = 0x01
    assert_eq!(bytes, vec![0xFF, 0x01]);
    let decoded: NineBools = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}
