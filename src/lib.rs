//! A serde-compatible serialization/deserialization library for the Blueberry
//! binary wire format.
//!
//! # Wire Format Summary
//!
//! - **Little-endian** byte order throughout
//! - Packets and messages always consist of multiples of 4-byte words
//! - Words are byte-aligned according to their size, **except** 8-byte types
//!   which use 4-byte alignment
//! - Consecutive boolean fields are bit-packed into shared bytes (LSb to MSb)
//! - Sequences use a 4-byte inline header (`u16 index` + `u16 elementByteLength`)
//!   with the actual data appended after the message body
//! - Strings use a 2-byte inline placeholder (`u16 index`) pointing to a
//!   deferred UTF-8 block (`u32 len + bytes`)
//! - Structs are packed inline; within sequence data blocks, struct fields have
//!   no alignment padding
//!
//! # Packet Structure
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │ Packet Header (8 bytes)                     │
//! │   0..4: Magic {'B','l','u','e'}             │
//! │   4..6: Total packet length in words (u16)  │
//! │   6..8: CRC-16-CCITT of message data (u16)  │
//! ├─────────────────────────────────────────────┤
//! │ Message 1 (header + body, 4-byte aligned)   │
//! ├─────────────────────────────────────────────┤
//! │ Message 2 (header + body, 4-byte aligned)   │
//! ├─────────────────────────────────────────────┤
//! │ ...                                         │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! # Message Header (8 bytes)
//!
//! ```text
//! Word 0: uint32 module_message_key
//! Word 1: uint16 length | uint8 fields_present | uint8 tbd
//! ```
//!
//! # Protocol
//!
//! - Operates in request-response mode on port 16962 (`0x4242`, `{'B','B'}`)
//! - One endpoint controls the bus and initiates requests; all other devices
//!   wait for requests before responding
//! - An empty message (header only) requests a populated response of the same
//!   type from the target device
//!
//! # Forward Compatibility
//!
//! Older firmware can deserialize messages from newer firmware: the deserializer
//! reads only the fields it knows about and silently skips any trailing fields
//! added in newer schema revisions.
//!
//! # Backward Compatibility
//!
//! Newer firmware can deserialize messages from older firmware by using
//! `Option<T>` for trailing struct fields. Fields not present in the message
//! (as determined by `max_ordinal` in the header) are returned as `None`.
//!
//! `Option<T>` fields **must** be at the trailing end of the struct. Once a
//! field is `None`, all subsequent fields must also be `None`.
//!
//! Supported patterns:
//!
//! ```rust
//! # use serde::{Serialize, Deserialize};
//! // Flat trailing optionals
//! #[derive(Serialize, Deserialize)]
//! struct Sensor {
//!     value: u32,
//!     status: u16,
//!     threshold: Option<u32>,
//! }
//!
//! // Extension struct pattern (recommended for versioning)
//! #[derive(Serialize, Deserialize)]
//! struct ExtV1 { c: u32 }
//!
//! #[derive(Serialize, Deserialize)]
//! struct Potato {
//!     a: u32,
//!     b: u32,
//!     ext_v1: Option<ExtV1>,
//! }
//! ```
//!
//! `Option<T>` inside sequences (`Vec<Option<T>>`) is not supported.
//!
//! # Examples
//!
//! ```rust
//! use blueberry_serde::{serialize, deserialize};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Point {
//!     x: u32,
//!     y: u32,
//! }
//!
//! let point = Point { x: 10, y: 20 };
//! let bytes = serialize(&point).unwrap();
//! let decoded: Point = deserialize(&bytes).unwrap();
//! assert_eq!(point, decoded);
//! ```
//!
//! ```rust
//! use blueberry_serde::{serialize_message, deserialize_message};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Status {
//!     code: u32,
//!     flags: u16,
//! }
//!
//! let status = Status { code: 42, flags: 0x01 };
//! let bytes = serialize_message(&status, 0x01, 0x02).unwrap();
//! let (header, decoded): (_, Status) = deserialize_message(&bytes).unwrap();
//! assert_eq!(status, decoded);
//! assert_eq!(header.module_key, 0x01);
//! assert_eq!(header.message_key, 0x02);
//! ```
//!
//! ```rust
//! use blueberry_serde::{
//!     serialize_message, serialize_packet, deserialize_packet, deserialize_message,
//! };
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Temp { value: u32 }
//!
//! let msg = serialize_message(&Temp { value: 42 }, 0x01, 0x01).unwrap();
//! let packet = serialize_packet(&[&msg]).unwrap();
//! let (pkt_hdr, messages) = deserialize_packet(&packet).unwrap();
//! let (msg_hdr, decoded): (_, Temp) = deserialize_message(messages[0]).unwrap();
//! assert_eq!(decoded, Temp { value: 42 });
//! ```

#![deny(warnings)]

pub mod de;
pub mod error;
pub mod header;
pub mod packet;
pub mod ser;

#[doc(inline)]
pub use crate::de::Deserializer;
#[doc(inline)]
pub use crate::error::{Error, Result};
#[doc(inline)]
pub use crate::header::{MessageHeader, HEADER_FIELD_COUNT, HEADER_SIZE};
#[doc(inline)]
pub use crate::packet::{
    crc16_ccitt, PacketHeader, BLUEBERRY_PORT, PACKET_HEADER_SIZE, PACKET_MAGIC,
};
#[doc(inline)]
pub use crate::ser::Serializer;

/// Serialize a value to bytes without a message header.
///
/// This is useful for raw data serialization or testing individual types.
pub fn serialize<T>(value: &T) -> Result<Vec<u8>>
where
    T: serde::Serialize + ?Sized,
{
    ser::serialize_data(value)
}

/// Deserialize a value from bytes without a message header.
pub fn deserialize<'de, T>(data: &'de [u8]) -> Result<T>
where
    T: serde::Deserialize<'de>,
{
    de::deserialize_data(data)
}

/// Serialize a value with a Blueberry message header.
///
/// The header includes:
/// - `module_key` and `message_key` combined into `module_message_key`
/// - `length`: total message size in 32-bit words
/// - `max_ordinal`: highest field ordinal present in the message
/// - `tbd`: reserved (set to 0)
pub fn serialize_message<T>(value: &T, module_key: u16, message_key: u16) -> Result<Vec<u8>>
where
    T: serde::Serialize + ?Sized,
{
    // Serialize the body first to determine field count and body size.
    // Set base_offset = HEADER_SIZE so sequence indices account for the header.
    let mut serializer = Serializer::new();
    serializer.set_base_offset(HEADER_SIZE);
    value.serialize(&mut serializer)?;
    let field_count = serializer.field_count();
    let body = serializer.finalize();

    // Build the complete message: header + body
    let total_bytes = HEADER_SIZE + body.len();
    // Pad to 4-byte boundary for the word count
    let padded_bytes = (total_bytes + 3) & !3;
    let length_words = (padded_bytes / 4) as u16;
    // Header uses ordinals 0..2, so payload fields start at ordinal 3.
    // Highest ordinal = 2 + field_count (or 2 when field_count == 0).
    let max_ordinal = (field_count as u8).saturating_add(HEADER_FIELD_COUNT.saturating_sub(1));

    let header = MessageHeader {
        module_key,
        message_key,
        length: length_words,
        max_ordinal,
        tbd: 0,
    };

    let mut result = Vec::with_capacity(padded_bytes);
    result.resize(HEADER_SIZE, 0);
    header.encode(&mut result[..HEADER_SIZE]);
    result.extend_from_slice(&body);
    // Pad to 4-byte boundary
    result.resize(padded_bytes, 0);

    Ok(result)
}

/// Deserialize a value from bytes that include a Blueberry message header.
///
/// Returns the parsed header and the deserialized value.
///
/// # Forward Compatibility
///
/// If the message contains more fields than the target type expects (e.g. a
/// newer schema), the extra trailing fields are silently ignored.
pub fn deserialize_message<'de, T>(data: &'de [u8]) -> Result<(MessageHeader, T)>
where
    T: serde::Deserialize<'de>,
{
    let header = MessageHeader::decode(data).ok_or(Error::InvalidHeader)?;
    let message_byte_len = header.length as usize * 4;
    let payload_field_count =
        (header.max_ordinal as usize).saturating_sub(HEADER_FIELD_COUNT.saturating_sub(1) as usize);

    let mut deserializer = Deserializer::with_message_context(data, HEADER_SIZE, message_byte_len);
    deserializer.set_payload_field_count(payload_field_count);
    let value = T::deserialize(&mut deserializer)?;

    Ok((header, value))
}

/// Create an empty message for use as a request in request-response mode.
///
/// In the Blueberry protocol, an empty message (containing only the 8-byte
/// message header with zero data fields) is sent to request a populated
/// response of the same message type from the target device.
pub fn empty_message(module_key: u16, message_key: u16) -> Vec<u8> {
    let header = MessageHeader {
        module_key,
        message_key,
        length: (HEADER_SIZE / 4) as u16,
        max_ordinal: HEADER_FIELD_COUNT.saturating_sub(1),
        tbd: 0,
    };
    let mut buf = vec![0u8; HEADER_SIZE];
    header.encode(&mut buf);
    buf
}

/// Pack one or more pre-serialized messages into a Blueberry packet.
///
/// Each message should already be serialized via [`serialize_message`] or
/// [`empty_message`]. The packet includes:
/// - 8-byte packet header (magic word `{'B','l','u','e'}`, length, CRC)
/// - All messages packed end-to-end
/// - Padding to ensure the packet is a multiple of 4 bytes
///
/// The CRC-16-CCITT is computed over all message data (everything after the
/// packet header), including any trailing padding bytes.
pub fn serialize_packet<M: AsRef<[u8]>>(messages: &[M]) -> Result<Vec<u8>> {
    use crate::packet::crc16_ccitt;

    let message_data_len: usize = messages.iter().map(|m| m.as_ref().len()).sum();
    let total_bytes = PACKET_HEADER_SIZE + message_data_len;
    let padded_bytes = (total_bytes + 3) & !3;
    let length_words = (padded_bytes / 4) as u16;

    let mut message_data = Vec::with_capacity(padded_bytes - PACKET_HEADER_SIZE);
    for msg in messages {
        message_data.extend_from_slice(msg.as_ref());
    }
    message_data.resize(padded_bytes - PACKET_HEADER_SIZE, 0);

    let crc = crc16_ccitt(&message_data);

    let pkt_header = PacketHeader { length_words, crc };

    let mut result = Vec::with_capacity(padded_bytes);
    result.resize(PACKET_HEADER_SIZE, 0);
    pkt_header.encode(&mut result[..PACKET_HEADER_SIZE]);
    result.extend_from_slice(&message_data);

    Ok(result)
}

/// Parse a Blueberry packet, returning the packet header and individual
/// message byte slices.
///
/// Validates the magic word, packet length, and CRC-16-CCITT. Each returned
/// slice contains a complete message (header + body) suitable for passing to
/// [`deserialize_message`].
pub fn deserialize_packet(data: &[u8]) -> Result<(PacketHeader, Vec<&[u8]>)> {
    let pkt_header = PacketHeader::decode(data).ok_or(Error::InvalidPacketHeader)?;

    let total_bytes = pkt_header.length_words as usize * 4;
    if data.len() < total_bytes {
        return Err(Error::UnexpectedEof);
    }

    let message_data = &data[PACKET_HEADER_SIZE..total_bytes];

    let expected_crc = crc16_ccitt(message_data);
    if pkt_header.crc != expected_crc {
        return Err(Error::CrcMismatch {
            expected: expected_crc,
            actual: pkt_header.crc,
        });
    }

    let mut messages = Vec::new();
    let mut offset = PACKET_HEADER_SIZE;
    while offset + HEADER_SIZE <= total_bytes {
        let msg_header = MessageHeader::decode(&data[offset..]).ok_or(Error::InvalidHeader)?;
        let msg_byte_len = msg_header.length as usize * 4;
        if msg_byte_len < HEADER_SIZE {
            break;
        }
        let msg_end = offset + msg_byte_len;
        if msg_end > total_bytes {
            return Err(Error::UnexpectedEof);
        }
        messages.push(&data[offset..msg_end]);
        offset = msg_end;
    }

    Ok((pkt_header, messages))
}
