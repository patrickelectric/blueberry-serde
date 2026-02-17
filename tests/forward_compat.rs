//! Tests for forward compatibility: older firmware can receive newer messages
//! and read only the fields they know about, ignoring trailing new fields.

use blueberry_serde::{deserialize_message, serialize_message};
use serde::{Deserialize, Serialize};

// "New" schema: has 4 fields
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct StatusV2 {
    code: u32,
    flags: u16,
    priority: u8,
    extra_data: u32,
}

// "Old" schema: only knows about 2 fields
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct StatusV1 {
    code: u32,
    flags: u16,
}

#[test]
fn old_firmware_reads_new_message() {
    // New firmware serializes V2
    let new_msg = StatusV2 {
        code: 42,
        flags: 0x01,
        priority: 5,
        extra_data: 9999,
    };
    let bytes = serialize_message(&new_msg, 0x01, 0x02).unwrap();

    // Old firmware deserializes as V1 (fewer fields)
    let (header, old_msg): (_, StatusV1) = deserialize_message(&bytes).unwrap();

    // Old firmware should successfully read the fields it knows
    assert_eq!(old_msg.code, 42);
    assert_eq!(old_msg.flags, 0x01);
    assert_eq!(header.module_key, 0x01);
    assert_eq!(header.message_key, 0x02);
}

// Even older: only knows about 1 field
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct StatusV0 {
    code: u32,
}

#[test]
fn very_old_firmware_reads_new_message() {
    let new_msg = StatusV2 {
        code: 123,
        flags: 0xFF,
        priority: 10,
        extra_data: 42,
    };
    let bytes = serialize_message(&new_msg, 0x10, 0x20).unwrap();

    let (header, old_msg): (_, StatusV0) = deserialize_message(&bytes).unwrap();
    assert_eq!(old_msg.code, 123);
    assert_eq!(header.module_key, 0x10);
}

#[test]
fn same_version_still_works() {
    let msg = StatusV2 {
        code: 1,
        flags: 2,
        priority: 3,
        extra_data: 4,
    };
    let bytes = serialize_message(&msg, 0, 0).unwrap();
    let (_header, decoded): (_, StatusV2) = deserialize_message(&bytes).unwrap();
    assert_eq!(msg, decoded);
}
