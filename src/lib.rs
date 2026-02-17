//! A serde-compatible serialization/deserialization library for the Blueberry
//! binary wire format.
//!
//! # Wire Format Summary
//!
//! - **Little-endian** byte order throughout
//! - Words are byte-aligned according to their size, **except** 8-byte types
//!   which use 4-byte alignment
//! - Messages have an 8-byte header: `module_message_key` (u32),
//!   `length` (u16), `max_ordinal` (u8), `tbd` (u8)
//! - Consecutive boolean fields are bit-packed into shared bytes (LSb to MSb)
//! - Sequences use a 4-byte inline header (`u16 index` + `u16 elementByteLength`)
//!   with the actual data appended after the message body
//! - Strings are UTF-8 byte sequences
//! - Structs are packed inline; within sequence data blocks, struct fields have
//!   no alignment padding
//!
//! # Forward Compatibility
//!
//! Older firmware can deserialize messages from newer firmware: the deserializer
//! reads only the fields it knows about and silently skips any trailing fields
//! added in newer schema revisions.
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

#![deny(warnings)]

pub mod de;
pub mod error;
pub mod header;
pub mod ser;

#[doc(inline)]
pub use crate::de::Deserializer;
#[doc(inline)]
pub use crate::error::{Error, Result};
#[doc(inline)]
pub use crate::header::{MessageHeader, HEADER_FIELD_COUNT, HEADER_SIZE};
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
/// - `max_ordinal`: total number of top-level fields (including 3 header fields)
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
    let max_ordinal = (field_count as u8).saturating_add(HEADER_FIELD_COUNT);

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

    let mut deserializer = Deserializer::with_message_context(data, HEADER_SIZE, message_byte_len);
    let value = T::deserialize(&mut deserializer)?;

    Ok((header, value))
}
