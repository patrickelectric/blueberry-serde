use byteorder::{ByteOrder, LittleEndian};

/// Message header for the Blueberry wire format.
///
/// Wire layout (8 bytes, 2 x 32-bit words):
/// ```text
/// Word 0: uint32 module_message_key
///         high bytes = module_key, low bytes = message_key
/// Word 1: uint16 length | uint8 max_ordinal | uint8 tbd
/// ```
///
/// All values are little-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageHeader {
    /// Module key (high 16 bits of module_message_key).
    pub module_key: u16,
    /// Message key (low 16 bits of module_message_key).
    pub message_key: u16,
    /// Total number of 32-bit words in this message (including header).
    pub length: u16,
    /// Highest field ordinal present in this message.
    ///
    /// Header ordinals are 0..2, so this is at least 2 for any valid message.
    pub max_ordinal: u8,
    /// Reserved for future use.
    pub tbd: u8,
}

/// Size of the message header in bytes.
pub const HEADER_SIZE: usize = 8;

/// Number of reserved header ordinals.
/// The header occupies ordinals 0..2.
pub const HEADER_FIELD_COUNT: u8 = 3;

impl MessageHeader {
    /// Encode the header into the first 8 bytes of `buf`.
    ///
    /// # Panics
    /// Panics if `buf.len() < 8`.
    pub fn encode(&self, buf: &mut [u8]) {
        assert!(buf.len() >= HEADER_SIZE);
        let module_message_key = ((self.module_key as u32) << 16) | (self.message_key as u32);
        LittleEndian::write_u32(&mut buf[0..4], module_message_key);
        LittleEndian::write_u16(&mut buf[4..6], self.length);
        buf[6] = self.max_ordinal;
        buf[7] = self.tbd;
    }

    /// Decode a header from the first 8 bytes of `buf`.
    ///
    /// Returns `None` if `buf` is too short.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < HEADER_SIZE {
            return None;
        }
        let module_message_key = LittleEndian::read_u32(&buf[0..4]);
        let module_key = (module_message_key >> 16) as u16;
        let message_key = (module_message_key & 0xFFFF) as u16;
        let length = LittleEndian::read_u16(&buf[4..6]);
        let max_ordinal = buf[6];
        let tbd = buf[7];
        Some(Self {
            module_key,
            message_key,
            length,
            max_ordinal,
            tbd,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_header() {
        let header = MessageHeader {
            module_key: 0x0102,
            message_key: 0x0304,
            length: 10,
            max_ordinal: 7,
            tbd: 0,
        };
        let mut buf = [0u8; 8];
        header.encode(&mut buf);
        let decoded = MessageHeader::decode(&buf).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn header_wire_layout() {
        let header = MessageHeader {
            module_key: 0x00AB,
            message_key: 0x00CD,
            length: 5,
            max_ordinal: 8,
            tbd: 0,
        };
        let mut buf = [0u8; 8];
        header.encode(&mut buf);
        // module_message_key = 0x00AB_00CD in LE
        assert_eq!(buf[0], 0xCD);
        assert_eq!(buf[1], 0x00);
        assert_eq!(buf[2], 0xAB);
        assert_eq!(buf[3], 0x00);
        // length = 5 in LE
        assert_eq!(buf[4], 0x05);
        assert_eq!(buf[5], 0x00);
        // max_ordinal
        assert_eq!(buf[6], 0x08);
        // tbd
        assert_eq!(buf[7], 0x00);
    }

    #[test]
    fn decode_too_short() {
        let buf = [0u8; 4];
        assert!(MessageHeader::decode(&buf).is_none());
    }
}
