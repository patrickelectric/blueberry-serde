use blueberry_serde::{deserialize_message, serialize_message, MessageHeader, HEADER_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SimpleMsg {
    code: u32,
    flags: u16,
}

#[test]
fn message_header_roundtrip() {
    let msg = SimpleMsg {
        code: 42,
        flags: 0x01,
    };
    let bytes = serialize_message(&msg, 0x00AB, 0x00CD).unwrap();

    // Parse header
    let header = MessageHeader::decode(&bytes).unwrap();
    assert_eq!(header.module_key, 0x00AB);
    assert_eq!(header.message_key, 0x00CD);
    assert_eq!(header.tbd, 0);

    // Deserialize
    let (hdr, decoded): (_, SimpleMsg) = deserialize_message(&bytes).unwrap();
    assert_eq!(msg, decoded);
    assert_eq!(hdr.module_key, 0x00AB);
    assert_eq!(hdr.message_key, 0x00CD);
}

#[test]
fn message_length_in_words() {
    let msg = SimpleMsg { code: 1, flags: 2 };
    let bytes = serialize_message(&msg, 0, 0).unwrap();
    let header = MessageHeader::decode(&bytes).unwrap();

    // Header = 8 bytes = 2 words
    // Body: u32(4) + u16(2) = 6 bytes, padded to 8 = 2 words
    // Total = 4 words
    assert_eq!(header.length, 4);
    assert_eq!(bytes.len(), 16); // 4 * 4
}

#[test]
fn message_max_ordinal_includes_header_fields() {
    let msg = SimpleMsg { code: 1, flags: 2 };
    let bytes = serialize_message(&msg, 0, 0).unwrap();
    let header = MessageHeader::decode(&bytes).unwrap();

    // SimpleMsg has 2 fields + 3 header fields = 5
    assert_eq!(header.max_ordinal, 5);
}

#[test]
fn message_starts_with_header_bytes() {
    let msg = SimpleMsg { code: 0, flags: 0 };
    let bytes = serialize_message(&msg, 0x01, 0x02).unwrap();

    // First 8 bytes are the header
    assert!(bytes.len() >= HEADER_SIZE);

    // module_message_key: (0x01 << 16) | 0x02 = 0x00010002 in LE
    assert_eq!(bytes[0], 0x02);
    assert_eq!(bytes[1], 0x00);
    assert_eq!(bytes[2], 0x01);
    assert_eq!(bytes[3], 0x00);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct EmptyMsg {}

#[test]
fn empty_message() {
    let msg = EmptyMsg {};
    let bytes = serialize_message(&msg, 0, 0).unwrap();
    let (header, decoded): (_, EmptyMsg) = deserialize_message(&bytes).unwrap();
    assert_eq!(msg, decoded);
    // Header only: 8 bytes = 2 words
    assert_eq!(header.length, 2);
    assert_eq!(header.max_ordinal, 3); // 0 fields + 3 header fields
}
