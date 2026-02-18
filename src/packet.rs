//! Packet-level framing for the Blueberry wire format.
//!
//! A Blueberry packet wraps one or more messages with an 8-byte packet header
//! that provides framing, length, and CRC integrity checking.
//!
//! # Packet Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │ Packet Header (8 bytes)                         │
//! │   Bytes 0..4: Magic {'B','l','u','e'}           │
//! │   Bytes 4..6: Total length in 4-byte words (LE) │
//! │   Bytes 6..8: CRC-16-CCITT of message data (LE) │
//! ├─────────────────────────────────────────────────┤
//! │ Message 1 (8-byte header + body, 4-byte aligned)│
//! ├─────────────────────────────────────────────────┤
//! │ Message 2 (8-byte header + body, 4-byte aligned)│
//! ├─────────────────────────────────────────────────┤
//! │ ...                                             │
//! ├─────────────────────────────────────────────────┤
//! │ Padding (if needed for 4-byte alignment)        │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! # Protocol
//!
//! Operates in request-response mode on port 16962 (`0x4242`, `{'B','B'}`).
//! One endpoint controls the bus and initiates requests; all other devices wait
//! for requests before responding. If an empty message (header only, no fields)
//! is received by a device, it responds with a populated message of the same
//! type, if it understands that type.

use byteorder::{ByteOrder, LittleEndian};

/// Magic start word for Blueberry packets: `{'B', 'l', 'u', 'e'}`.
pub const PACKET_MAGIC: [u8; 4] = [0x42, 0x6c, 0x75, 0x65];

/// Size of the packet header in bytes.
pub const PACKET_HEADER_SIZE: usize = 8;

/// Default Blueberry protocol port: 16962 (`0x4242`, `{'B', 'B'}`).
pub const BLUEBERRY_PORT: u16 = 16962;

/// Packet header for the Blueberry wire format.
///
/// Wire layout (8 bytes, 2 × 32-bit words):
///
/// ```text
/// Bytes 0..4: Magic {'B','l','u','e'} = {0x42, 0x6c, 0x75, 0x65}
/// Bytes 4..6: uint16 total packet length in 4-byte words (LE)
/// Bytes 6..8: uint16 CRC-16-CCITT of message data + padding (LE)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    /// Total packet length in 4-byte words (includes the packet header itself).
    pub length_words: u16,
    /// CRC-16-CCITT computed over all message data after the packet header,
    /// including any trailing padding bytes.
    pub crc: u16,
}

impl PacketHeader {
    /// Encode the packet header into the first 8 bytes of `buf`.
    ///
    /// # Panics
    /// Panics if `buf.len() < 8`.
    pub fn encode(&self, buf: &mut [u8]) {
        assert!(buf.len() >= PACKET_HEADER_SIZE);
        buf[0..4].copy_from_slice(&PACKET_MAGIC);
        LittleEndian::write_u16(&mut buf[4..6], self.length_words);
        LittleEndian::write_u16(&mut buf[6..8], self.crc);
    }

    /// Decode a packet header from the first 8 bytes of `buf`.
    ///
    /// Returns `None` if `buf` is too short or the magic word doesn't match.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < PACKET_HEADER_SIZE {
            return None;
        }
        if buf[0..4] != PACKET_MAGIC {
            return None;
        }
        let length_words = LittleEndian::read_u16(&buf[4..6]);
        let crc = LittleEndian::read_u16(&buf[6..8]);
        Some(Self { length_words, crc })
    }
}

/// Compute CRC-16-CCITT (CCITT-FALSE variant) over a byte slice.
///
/// - Polynomial: `0x1021`
/// - Initial value: `0xFFFF`
/// - No input/output bit reflection
/// - No final XOR
pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_packet_header() {
        let header = PacketHeader {
            length_words: 42,
            crc: 0xABCD,
        };
        let mut buf = [0u8; 8];
        header.encode(&mut buf);
        let decoded = PacketHeader::decode(&buf).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn packet_header_wire_layout() {
        let header = PacketHeader {
            length_words: 10,
            crc: 0x1234,
        };
        let mut buf = [0u8; 8];
        header.encode(&mut buf);
        assert_eq!(&buf[0..4], &PACKET_MAGIC);
        assert_eq!(buf[4], 0x0A);
        assert_eq!(buf[5], 0x00);
        assert_eq!(buf[6], 0x34);
        assert_eq!(buf[7], 0x12);
    }

    #[test]
    fn decode_bad_magic() {
        let buf = [0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x34, 0x12];
        assert!(PacketHeader::decode(&buf).is_none());
    }

    #[test]
    fn decode_too_short() {
        let buf = [0x42, 0x6c, 0x75];
        assert!(PacketHeader::decode(&buf).is_none());
    }

    #[test]
    fn crc_empty_data() {
        assert_eq!(crc16_ccitt(&[]), 0xFFFF);
    }

    #[test]
    fn crc_known_vector() {
        // "123456789" produces 0x29B1 for CRC-16/CCITT-FALSE
        assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
    }
}
