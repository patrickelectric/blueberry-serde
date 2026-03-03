//! Tests for `Option<T>` trailing field support (backward compatibility).
//!
//! Newer firmware can use `Option<T>` at the trailing end of structs to
//! gracefully handle messages from older firmware that lack those fields.

use blueberry_serde::{deserialize, deserialize_message, serialize, serialize_message};
use serde::{Deserialize, Serialize};

// Flat sender types (simulate old firmware without Options)
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PotatoBase {
    a: u32,
    b: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PotatoV1Flat {
    a: u32,
    b: u32,
    c: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PotatoFullFlat {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u16,
}

// Extension structs
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ExtV1 {
    c: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ExtV2 {
    d: u32,
    e: u16,
}

// Receiver with flat trailing Option
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PotatoOpt {
    a: u32,
    b: u32,
    c: Option<u32>,
}

// Receiver with chained extension Options
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PotatoChained {
    a: u32,
    b: u32,
    ext_v1: Option<ExtV1>,
    ext_v2: Option<ExtV2>,
}

// Receiver with multiple flat Options
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MultiOpt {
    a: u32,
    b: Option<u32>,
    c: Option<u32>,
}

// Extension struct receiver
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PotatoExtOpt {
    a: u32,
    b: u32,
    ext: Option<ExtV1>,
}

#[test]
fn option_some_roundtrip_raw() {
    let val = PotatoOpt {
        a: 10,
        b: 20,
        c: Some(30),
    };
    let bytes = serialize(&val).unwrap();
    assert_eq!(
        bytes,
        vec![
            0x0A, 0x00, 0x00, 0x00, // a = 10
            0x14, 0x00, 0x00, 0x00, // b = 20
            0x1E, 0x00, 0x00, 0x00, // c = 30
        ]
    );
    let decoded: PotatoOpt = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn option_none_roundtrip_raw() {
    let val = PotatoOpt {
        a: 10,
        b: 20,
        c: None,
    };
    let bytes = serialize(&val).unwrap();
    assert_eq!(
        bytes,
        vec![
            0x0A, 0x00, 0x00, 0x00, // a = 10
            0x14, 0x00, 0x00, 0x00, // b = 20
                  // c = None (nothing written)
        ]
    );
    let decoded: PotatoOpt = deserialize(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn option_none_from_short_data_raw() {
    let base = PotatoBase { a: 10, b: 20 };
    let bytes = serialize(&base).unwrap();
    let decoded: PotatoOpt = deserialize(&bytes).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
    assert_eq!(decoded.c, None);
}

#[test]
fn option_some_roundtrip_message() {
    let val = PotatoOpt {
        a: 10,
        b: 20,
        c: Some(30),
    };
    let bytes = serialize_message(&val, 0x01, 0x02).unwrap();
    let (header, decoded): (_, PotatoOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
    assert_eq!(header.module_key, 0x01);
    assert_eq!(header.message_key, 0x02);
    assert_eq!(header.max_ordinal, 2 + 3); // 3 top-level fields + 2
}

#[test]
fn option_none_roundtrip_message() {
    let val = PotatoOpt {
        a: 10,
        b: 20,
        c: None,
    };
    let bytes = serialize_message(&val, 0x01, 0x02).unwrap();
    let (header, decoded): (_, PotatoOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
    assert_eq!(header.max_ordinal, 2 + 2); // 2 top-level fields written + 2
}

#[test]
fn backward_compat_base_to_opt() {
    let base = PotatoBase { a: 10, b: 20 };
    let bytes = serialize_message(&base, 0x01, 0x02).unwrap();
    let (_, decoded): (_, PotatoOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
    assert_eq!(decoded.c, None);
}

#[test]
fn forward_compat_opt_to_base() {
    let val = PotatoOpt {
        a: 10,
        b: 20,
        c: Some(30),
    };
    let bytes = serialize_message(&val, 0x01, 0x02).unwrap();
    let (_, decoded): (_, PotatoBase) = deserialize_message(&bytes).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
}

#[test]
fn extension_struct_some() {
    let val = PotatoExtOpt {
        a: 10,
        b: 20,
        ext: Some(ExtV1 { c: 30 }),
    };
    let bytes = serialize_message(&val, 0x01, 0x02).unwrap();
    let (_, decoded): (_, PotatoExtOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn extension_struct_none() {
    let val = PotatoExtOpt {
        a: 10,
        b: 20,
        ext: None,
    };
    let bytes = serialize_message(&val, 0x01, 0x02).unwrap();
    let (_, decoded): (_, PotatoExtOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn extension_backward_compat() {
    let base = PotatoBase { a: 10, b: 20 };
    let bytes = serialize_message(&base, 0x01, 0x02).unwrap();
    let (_, decoded): (_, PotatoExtOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
    assert_eq!(decoded.ext, None);
}

#[test]
fn multiple_options_all_some() {
    let val = MultiOpt {
        a: 1,
        b: Some(2),
        c: Some(3),
    };
    let bytes = serialize_message(&val, 0, 0).unwrap();
    let (_, decoded): (_, MultiOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn multiple_options_partial() {
    let val = MultiOpt {
        a: 1,
        b: Some(2),
        c: None,
    };
    let bytes = serialize_message(&val, 0, 0).unwrap();
    let (_, decoded): (_, MultiOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn multiple_options_all_none() {
    let val = MultiOpt {
        a: 1,
        b: None,
        c: None,
    };
    let bytes = serialize_message(&val, 0, 0).unwrap();
    let (_, decoded): (_, MultiOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn multiple_options_backward_compat() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct JustA {
        a: u32,
    }
    let base = JustA { a: 42 };
    let bytes = serialize_message(&base, 0, 0).unwrap();
    let (_, decoded): (_, MultiOpt) = deserialize_message(&bytes).unwrap();
    assert_eq!(decoded.a, 42);
    assert_eq!(decoded.b, None);
    assert_eq!(decoded.c, None);
}

const MODULE_KEY: u16 = 0x01;
const MESSAGE_KEY: u16 = 0x02;

#[rustfmt::skip]
const GOLD_BASE: [u8; 16] = [
    0x02, 0x00, 0x01, 0x00, // header word 0: module_message_key
    0x04, 0x00, 0x04, 0x00, // header word 1: length=4, max_ordinal=4, tbd=0
    0x0A, 0x00, 0x00, 0x00, // a = 10
    0x14, 0x00, 0x00, 0x00, // b = 20
];

#[rustfmt::skip]
const GOLD_V1: [u8; 20] = [
    0x02, 0x00, 0x01, 0x00, // header word 0
    0x05, 0x00, 0x05, 0x00, // header word 1: length=5, max_ordinal=5, tbd=0
    0x0A, 0x00, 0x00, 0x00, // a = 10
    0x14, 0x00, 0x00, 0x00, // b = 20
    0x1E, 0x00, 0x00, 0x00, // c = 30
];

#[rustfmt::skip]
const GOLD_FULL: [u8; 28] = [
    0x02, 0x00, 0x01, 0x00, // header word 0
    0x07, 0x00, 0x07, 0x00, // header word 1: length=7, max_ordinal=7, tbd=0
    0x0A, 0x00, 0x00, 0x00, // a = 10
    0x14, 0x00, 0x00, 0x00, // b = 20
    0x1E, 0x00, 0x00, 0x00, // c = 30
    0x28, 0x00, 0x00, 0x00, // d = 40
    0x32, 0x00, 0x00, 0x00, // e = 50 (u16) + 2 bytes word-alignment padding
];

#[test]
fn gold_base_deserialize_as_chained() {
    let (_, decoded): (_, PotatoChained) = deserialize_message(&GOLD_BASE).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
    assert_eq!(decoded.ext_v1, None);
    assert_eq!(decoded.ext_v2, None);
}

#[test]
fn gold_v1_deserialize_as_chained() {
    let (_, decoded): (_, PotatoChained) = deserialize_message(&GOLD_V1).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
    assert_eq!(decoded.ext_v1, Some(ExtV1 { c: 30 }));
    assert_eq!(decoded.ext_v2, None);
}

#[test]
fn gold_v1_deserialize_as_base() {
    let (_, decoded): (_, PotatoBase) = deserialize_message(&GOLD_V1).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
}

#[test]
fn gold_full_deserialize_as_chained() {
    let (_, decoded): (_, PotatoChained) = deserialize_message(&GOLD_FULL).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
    assert_eq!(decoded.ext_v1, Some(ExtV1 { c: 30 }));
    assert_eq!(decoded.ext_v2, Some(ExtV2 { d: 40, e: 50 }));
}

#[test]
fn gold_full_deserialize_as_base() {
    let (_, decoded): (_, PotatoBase) = deserialize_message(&GOLD_FULL).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
}

#[test]
fn gold_full_deserialize_as_v1() {
    let (_, decoded): (_, PotatoV1Flat) = deserialize_message(&GOLD_FULL).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, 20);
    assert_eq!(decoded.c, 30);
}

#[test]
fn gold_base_verify_serialization() {
    let val = PotatoBase { a: 10, b: 20 };
    let bytes = serialize_message(&val, MODULE_KEY, MESSAGE_KEY).unwrap();
    assert_eq!(&bytes[..], &GOLD_BASE[..]);
}

#[test]
fn gold_v1_verify_serialization() {
    let val = PotatoV1Flat {
        a: 10,
        b: 20,
        c: 30,
    };
    let bytes = serialize_message(&val, MODULE_KEY, MESSAGE_KEY).unwrap();
    assert_eq!(&bytes[..], &GOLD_V1[..]);
}

#[test]
fn gold_full_verify_serialization() {
    let val = PotatoFullFlat {
        a: 10,
        b: 20,
        c: 30,
        d: 40,
        e: 50,
    };
    let bytes = serialize_message(&val, MODULE_KEY, MESSAGE_KEY).unwrap();
    assert_eq!(&bytes[..], &GOLD_FULL[..]);
}

#[test]
fn chained_serialize_v1_matches_gold() {
    let val = PotatoChained {
        a: 10,
        b: 20,
        ext_v1: Some(ExtV1 { c: 30 }),
        ext_v2: None,
    };
    let bytes = serialize_message(&val, MODULE_KEY, MESSAGE_KEY).unwrap();
    // Body should match gold V1 body (skip header comparison since
    // max_ordinal differs due to Option struct counting)
    assert_eq!(&bytes[8..], &GOLD_V1[8..]);
}

#[test]
fn chained_serialize_base_matches_gold() {
    let val = PotatoChained {
        a: 10,
        b: 20,
        ext_v1: None,
        ext_v2: None,
    };
    let bytes = serialize_message(&val, MODULE_KEY, MESSAGE_KEY).unwrap();
    assert_eq!(&bytes[..], &GOLD_BASE[..]);
}

#[test]
fn chained_full_roundtrip() {
    let val = PotatoChained {
        a: 10,
        b: 20,
        ext_v1: Some(ExtV1 { c: 30 }),
        ext_v2: Some(ExtV2 { d: 40, e: 50 }),
    };
    let bytes = serialize_message(&val, MODULE_KEY, MESSAGE_KEY).unwrap();
    let (_, decoded): (_, PotatoChained) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn empty_struct_with_option() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct OnlyOpt {
        x: Option<u32>,
    }

    let empty_bytes = blueberry_serde::empty_message(0, 0);
    let (_, decoded): (_, OnlyOpt) = deserialize_message(&empty_bytes).unwrap();
    assert_eq!(decoded.x, None);
}

#[test]
fn option_with_u16_trailing_message() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct WithU16Opt {
        a: u32,
        b: Option<u16>,
    }

    let val_some = WithU16Opt { a: 10, b: Some(42) };
    let bytes = serialize_message(&val_some, 0, 0).unwrap();
    let (_, decoded): (_, WithU16Opt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val_some, decoded);

    let val_none = WithU16Opt { a: 10, b: None };
    let bytes = serialize_message(&val_none, 0, 0).unwrap();
    let (_, decoded): (_, WithU16Opt) = deserialize_message(&bytes).unwrap();
    assert_eq!(val_none, decoded);

    // Backward compat: base struct -> struct with optional u16
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct JustA {
        a: u32,
    }
    let base = JustA { a: 10 };
    let bytes = serialize_message(&base, 0, 0).unwrap();
    let (_, decoded): (_, WithU16Opt) = deserialize_message(&bytes).unwrap();
    assert_eq!(decoded.a, 10);
    assert_eq!(decoded.b, None);
}

// Fields c, d, e (u8) are placed after a (u8) and before b (u32), filling
// the alignment gap. V1-V3 produce the same 16-byte message as base; only
// max_ordinal differs. f (u32) is truly trailing after b.

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SmallPotato {
    a: u8,
    b: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SmallPotatoV1Flat {
    a: u8,
    c: u8,
    b: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SmallPotatoV2Flat {
    a: u8,
    c: u8,
    d: u8,
    b: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SmallPotatoV3Flat {
    a: u8,
    c: u8,
    d: u8,
    e: u8,
    b: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SmallPotatoV4Flat {
    a: u8,
    c: u8,
    d: u8,
    e: u8,
    b: u32,
    f: u32,
}

const SP_MODULE: u16 = 0x01;
const SP_MSG: u16 = 0x02;

// All versions base through V3 share the same message size (16 bytes / 4 words).
// Only max_ordinal distinguishes how many of the padding-gap fields are valid.

#[rustfmt::skip]
const SP_GOLD_BASE: [u8; 16] = [
    0x02, 0x00, 0x01, 0x00, // header word 0: module_message_key
    0x04, 0x00, 0x04, 0x00, // header word 1: length=4, max_ordinal=4, tbd=0
    0x01, 0x00, 0x00, 0x00, // a=1 + 3 bytes alignment padding for b
    0x02, 0x00, 0x00, 0x00, // b=2
];

#[rustfmt::skip]
const SP_GOLD_V1: [u8; 16] = [
    0x02, 0x00, 0x01, 0x00, // header word 0
    0x04, 0x00, 0x05, 0x00, // header word 1: length=4, max_ordinal=5, tbd=0
    0x01, 0x03, 0x00, 0x00, // a=1, c=3 + 2 bytes padding for b
    0x02, 0x00, 0x00, 0x00, // b=2
];

#[rustfmt::skip]
const SP_GOLD_V2: [u8; 16] = [
    0x02, 0x00, 0x01, 0x00, // header word 0
    0x04, 0x00, 0x06, 0x00, // header word 1: length=4, max_ordinal=6, tbd=0
    0x01, 0x03, 0x04, 0x00, // a=1, c=3, d=4 + 1 byte padding for b
    0x02, 0x00, 0x00, 0x00, // b=2
];

#[rustfmt::skip]
const SP_GOLD_V3: [u8; 16] = [
    0x02, 0x00, 0x01, 0x00, // header word 0
    0x04, 0x00, 0x07, 0x00, // header word 1: length=4, max_ordinal=7, tbd=0
    0x01, 0x03, 0x04, 0x05, // a=1, c=3, d=4, e=5 (fills gap exactly)
    0x02, 0x00, 0x00, 0x00, // b=2
];

#[rustfmt::skip]
const SP_GOLD_V4: [u8; 20] = [
    0x02, 0x00, 0x01, 0x00, // header word 0
    0x05, 0x00, 0x08, 0x00, // header word 1: length=5, max_ordinal=8, tbd=0
    0x01, 0x03, 0x04, 0x05, // a=1, c=3, d=4, e=5
    0x02, 0x00, 0x00, 0x00, // b=2
    0x06, 0x00, 0x00, 0x00, // f=6
];

// -- Gold-wired serialization verification --

#[test]
fn sp_gold_base_serialize() {
    let val = SmallPotato { a: 1, b: 2 };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    assert_eq!(&bytes[..], &SP_GOLD_BASE[..]);
}

#[test]
fn sp_gold_v1_serialize() {
    let val = SmallPotatoV1Flat { a: 1, c: 3, b: 2 };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    assert_eq!(&bytes[..], &SP_GOLD_V1[..]);
}

#[test]
fn sp_gold_v2_serialize() {
    let val = SmallPotatoV2Flat {
        a: 1,
        c: 3,
        d: 4,
        b: 2,
    };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    assert_eq!(&bytes[..], &SP_GOLD_V2[..]);
}

#[test]
fn sp_gold_v3_serialize() {
    let val = SmallPotatoV3Flat {
        a: 1,
        c: 3,
        d: 4,
        e: 5,
        b: 2,
    };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    assert_eq!(&bytes[..], &SP_GOLD_V3[..]);
}

#[test]
fn sp_gold_v4_serialize() {
    let val = SmallPotatoV4Flat {
        a: 1,
        c: 3,
        d: 4,
        e: 5,
        b: 2,
        f: 6,
    };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    assert_eq!(&bytes[..], &SP_GOLD_V4[..]);
}

// -- Forward compat: newer message -> older struct --
// Alignment naturally skips over interleaved fields when reading b (u32).

#[test]
fn sp_v4_as_base() {
    let (_, d): (_, SmallPotato) = deserialize_message(&SP_GOLD_V4).unwrap();
    assert_eq!(d.a, 1);
    assert_eq!(d.b, 2);
}

#[test]
fn sp_v4_as_v1() {
    let (_, d): (_, SmallPotatoV1Flat) = deserialize_message(&SP_GOLD_V4).unwrap();
    assert_eq!(d.a, 1);
    assert_eq!(d.c, 3);
    assert_eq!(d.b, 2);
}

#[test]
fn sp_v4_as_v2() {
    let (_, d): (_, SmallPotatoV2Flat) = deserialize_message(&SP_GOLD_V4).unwrap();
    assert_eq!(d.a, 1);
    assert_eq!(d.c, 3);
    assert_eq!(d.d, 4);
    assert_eq!(d.b, 2);
}

#[test]
fn sp_v4_as_v3() {
    let (_, d): (_, SmallPotatoV3Flat) = deserialize_message(&SP_GOLD_V4).unwrap();
    assert_eq!(d.a, 1);
    assert_eq!(d.c, 3);
    assert_eq!(d.d, 4);
    assert_eq!(d.e, 5);
    assert_eq!(d.b, 2);
}

#[test]
fn sp_v3_as_base() {
    let (_, d): (_, SmallPotato) = deserialize_message(&SP_GOLD_V3).unwrap();
    assert_eq!(d.a, 1);
    assert_eq!(d.b, 2);
}

#[test]
fn sp_v4_roundtrip() {
    let val = SmallPotatoV4Flat {
        a: 1,
        c: 3,
        d: 4,
        e: 5,
        b: 2,
        f: 6,
    };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    let (_, d): (_, SmallPotatoV4Flat) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, d);
}

#[test]
fn sp_v3_roundtrip() {
    let val = SmallPotatoV3Flat {
        a: 1,
        c: 3,
        d: 4,
        e: 5,
        b: 2,
    };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    let (_, d): (_, SmallPotatoV3Flat) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, d);
}

#[test]
fn sp_v1_roundtrip() {
    let val = SmallPotatoV1Flat { a: 1, c: 3, b: 2 };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    let (_, d): (_, SmallPotatoV1Flat) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, d);
}

#[test]
fn sp_base_roundtrip() {
    let val = SmallPotato { a: 1, b: 2 };
    let bytes = serialize_message(&val, SP_MODULE, SP_MSG).unwrap();
    let (_, d): (_, SmallPotato) = deserialize_message(&bytes).unwrap();
    assert_eq!(val, d);
}
