//! Deserializing the Blueberry wire format into Rust data types.

use byteorder::{ByteOrder, LittleEndian};
use serde::de::{self, IntoDeserializer};

use crate::error::{Error, Result};

/// A deserializer that reads from a byte slice using the Blueberry wire format.
///
/// Supports forward-compatibility: when a message contains more fields than the
/// target struct expects, the extra fields are silently skipped.
pub struct Deserializer<'de> {
    /// Full message data.
    data: &'de [u8],
    /// Current read position.
    pos: usize,
    /// Boolean unpacking state: current bit position (0..8).
    bool_bit_pos: u8,
    /// Boolean unpacking state: the byte currently being unpacked.
    bool_byte: Option<u8>,
    /// Whether we are inside a sequence data block (suppresses alignment).
    in_seq_data: bool,
    /// Message length in bytes (set when deserializing with header, used for
    /// forward-compat skip). `None` for raw deserialization.
    message_byte_len: Option<usize>,
    /// Position where the message body starts (after header).
    message_start: usize,
}

impl<'de> Deserializer<'de> {
    /// Create a new deserializer from a byte slice (raw, no header).
    pub fn new(data: &'de [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bool_bit_pos: 0,
            bool_byte: None,
            in_seq_data: false,
            message_byte_len: None,
            message_start: 0,
        }
    }

    /// Create a new deserializer positioned after a message header.
    ///
    /// `message_byte_len` is the total message size in bytes (including header).
    /// `body_start` is the byte offset where the body begins (after header).
    pub fn with_message_context(
        data: &'de [u8],
        body_start: usize,
        message_byte_len: usize,
    ) -> Self {
        Self {
            data,
            pos: body_start,
            bool_bit_pos: 0,
            bool_byte: None,
            in_seq_data: false,
            message_byte_len: Some(message_byte_len),
            message_start: 0,
        }
    }

    /// Skip forward to the end of the message (for forward-compat).
    fn skip_to_message_end(&mut self) {
        if let Some(len) = self.message_byte_len {
            let end = self.message_start + len;
            if self.pos < end {
                self.pos = end;
            }
        }
    }

    /// Read alignment padding for a value of the given size.
    fn read_padding(&mut self, size: usize) {
        if self.in_seq_data || size <= 1 {
            return;
        }
        let align = if size >= 8 { 4 } else { size };
        let rem = self.pos % align;
        if rem != 0 {
            let pad = align - rem;
            self.pos += pad;
        }
    }

    /// Flush boolean unpacking state.
    fn flush_bools(&mut self) {
        self.bool_bit_pos = 0;
        self.bool_byte = None;
    }

    /// Check that we have at least `n` bytes remaining.
    fn check_remaining(&self, n: usize) -> Result<()> {
        if self.pos + n > self.data.len() {
            Err(Error::UnexpectedEof)
        } else {
            Ok(())
        }
    }

    fn read_bool(&mut self) -> Result<bool> {
        if let Some(byte) = self.bool_byte {
            let v = (byte >> self.bool_bit_pos) & 1 != 0;
            self.bool_bit_pos += 1;
            if self.bool_bit_pos >= 8 {
                self.bool_bit_pos = 0;
                self.bool_byte = None;
            }
            Ok(v)
        } else {
            // Read a new byte
            self.read_padding(1);
            self.check_remaining(1)?;
            let byte = self.data[self.pos];
            self.pos += 1;
            let v = byte & 1 != 0;
            self.bool_bit_pos = 1;
            self.bool_byte = Some(byte);
            Ok(v)
        }
    }

    fn read_u8(&mut self) -> Result<u8> {
        self.flush_bools();
        self.read_padding(1);
        self.check_remaining(1)?;
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    fn read_u16(&mut self) -> Result<u16> {
        self.flush_bools();
        self.read_padding(2);
        self.check_remaining(2)?;
        let v = LittleEndian::read_u16(&self.data[self.pos..]);
        self.pos += 2;
        Ok(v)
    }

    fn read_i16(&mut self) -> Result<i16> {
        self.flush_bools();
        self.read_padding(2);
        self.check_remaining(2)?;
        let v = LittleEndian::read_i16(&self.data[self.pos..]);
        self.pos += 2;
        Ok(v)
    }

    fn read_u32(&mut self) -> Result<u32> {
        self.flush_bools();
        self.read_padding(4);
        self.check_remaining(4)?;
        let v = LittleEndian::read_u32(&self.data[self.pos..]);
        self.pos += 4;
        Ok(v)
    }

    fn read_i32(&mut self) -> Result<i32> {
        self.flush_bools();
        self.read_padding(4);
        self.check_remaining(4)?;
        let v = LittleEndian::read_i32(&self.data[self.pos..]);
        self.pos += 4;
        Ok(v)
    }

    fn read_u64(&mut self) -> Result<u64> {
        self.flush_bools();
        self.read_padding(8);
        self.check_remaining(8)?;
        let v = LittleEndian::read_u64(&self.data[self.pos..]);
        self.pos += 8;
        Ok(v)
    }

    fn read_i64(&mut self) -> Result<i64> {
        self.flush_bools();
        self.read_padding(8);
        self.check_remaining(8)?;
        let v = LittleEndian::read_i64(&self.data[self.pos..]);
        self.pos += 8;
        Ok(v)
    }

    fn read_f32(&mut self) -> Result<f32> {
        self.flush_bools();
        self.read_padding(4);
        self.check_remaining(4)?;
        let v = LittleEndian::read_f32(&self.data[self.pos..]);
        self.pos += 4;
        Ok(v)
    }

    fn read_f64(&mut self) -> Result<f64> {
        self.flush_bools();
        self.read_padding(8);
        self.check_remaining(8)?;
        let v = LittleEndian::read_f64(&self.data[self.pos..]);
        self.pos += 8;
        Ok(v)
    }

    /// Read a sequence header (u16 index, u16 elementByteLength) and return
    /// the data block as a slice.
    fn read_sequence_header(&mut self) -> Result<(u16, u16)> {
        self.flush_bools();
        self.read_padding(2);
        self.check_remaining(4)?;
        let index = LittleEndian::read_u16(&self.data[self.pos..]);
        let elem_byte_len = LittleEndian::read_u16(&self.data[self.pos + 2..]);
        self.pos += 4;
        Ok((index, elem_byte_len))
    }

    /// Read a string placeholder (u16 index into deferred data block).
    fn read_string_index(&mut self) -> Result<u16> {
        self.flush_bools();
        self.read_padding(2);
        self.check_remaining(2)?;
        let index = LittleEndian::read_u16(&self.data[self.pos..]);
        self.pos += 2;
        Ok(index)
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::DeserializeAnyNotSupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_bool(self.read_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i8(self.read_i8()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i16(self.read_i16()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i32(self.read_i32()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i64(self.read_i64()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u8(self.read_u8()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u16(self.read_u16()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u32(self.read_u32()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u64(self.read_u64()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_f32(self.read_f32()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_f64(self.read_f64()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // Deserialize as a string and take the first char
        let s: String = de::Deserialize::deserialize(&mut *self)?;
        let c = s.chars().next().ok_or(Error::UnexpectedEof)?;
        visitor.visit_char(c)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // String placeholder = u16 index into deferred UTF-8 block.
        let index = self.read_string_index()?;

        if index == 0 {
            // Zero header = empty string
            return visitor.visit_borrowed_str("");
        }

        let data_start = self.message_start + index as usize;
        if data_start + 4 > self.data.len() {
            return Err(Error::SequenceIndexOutOfBounds(data_start));
        }

        let count = LittleEndian::read_u32(&self.data[data_start..]) as usize;
        let bytes_start = data_start + 4;
        let bytes_end = bytes_start + count;

        if bytes_end > self.data.len() {
            return Err(Error::SequenceIndexOutOfBounds(bytes_end));
        }

        let s = std::str::from_utf8(&self.data[bytes_start..bytes_end])?;
        visitor.visit_borrowed_str(s)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let (index, _elem_byte_len) = self.read_sequence_header()?;

        if index == 0 {
            return visitor.visit_borrowed_bytes(&[]);
        }

        let data_start = self.message_start + index as usize;
        if data_start + 4 > self.data.len() {
            return Err(Error::SequenceIndexOutOfBounds(data_start));
        }

        let count = LittleEndian::read_u32(&self.data[data_start..]) as usize;
        let bytes_start = data_start + 4;
        let bytes_end = bytes_start + count;

        if bytes_end > self.data.len() {
            return Err(Error::SequenceIndexOutOfBounds(bytes_end));
        }

        visitor.visit_borrowed_bytes(&self.data[bytes_start..bytes_end])
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::TypeNotSupported)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.flush_bools();
        let (index, elem_byte_len) = self.read_sequence_header()?;

        if index == 0 && elem_byte_len == 0 {
            // Zero header = empty sequence
            return visitor.visit_seq(SequenceAccess {
                de: self,
                remaining: 0,
                data_pos: 0,
                _elem_byte_len: 0,
                in_data_block: false,
            });
        }

        let data_start = self.message_start + index as usize;
        if data_start + 4 > self.data.len() {
            return Err(Error::SequenceIndexOutOfBounds(data_start));
        }

        let count = LittleEndian::read_u32(&self.data[data_start..]) as usize;
        let elements_start = data_start + 4;

        visitor.visit_seq(SequenceAccess {
            de: self,
            remaining: count,
            data_pos: elements_start,
            _elem_byte_len: elem_byte_len as usize,
            in_data_block: true,
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.flush_bools();
        visitor.visit_seq(StructAccess {
            de: self,
            remaining: len,
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::TypeNotSupported)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.flush_bools();
        visitor.visit_seq(StructAccess {
            de: self,
            remaining: fields.len(),
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::TypeNotSupported)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // In forward-compat scenarios, we skip unknown trailing data via
        // skip_to_message_end rather than parsing individual unknown fields.
        visitor.visit_unit()
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// SeqAccess for inline struct/tuple fields.
struct StructAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> de::SeqAccess<'de> for StructAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            // Forward-compat: skip any remaining message data
            self.de.skip_to_message_end();
            return Ok(None);
        }
        self.remaining -= 1;
        let value = de::DeserializeSeed::deserialize(seed, &mut *self.de)?;
        Ok(Some(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

/// SeqAccess for sequence data blocks (deferred data at end of message).
struct SequenceAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
    data_pos: usize,
    _elem_byte_len: usize,
    in_data_block: bool,
}

impl<'a, 'de> de::SeqAccess<'de> for SequenceAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;

        if self.in_data_block {
            // Save and redirect the deserializer to read from the data block
            let saved_pos = self.de.pos;
            let saved_in_seq = self.de.in_seq_data;
            self.de.pos = self.data_pos;
            self.de.in_seq_data = true;
            self.de.flush_bools();

            let value = de::DeserializeSeed::deserialize(seed, &mut *self.de)?;

            self.data_pos = self.de.pos;
            self.de.pos = saved_pos;
            self.de.in_seq_data = saved_in_seq;

            Ok(Some(value))
        } else {
            let value = de::DeserializeSeed::deserialize(seed, &mut *self.de)?;
            Ok(Some(value))
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

/// EnumAccess implementation.
impl<'de> de::EnumAccess<'de> for &mut Deserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let idx = self.read_u32()?;
        let val: Result<_> = seed.deserialize(idx.into_deserializer());
        Ok((val?, self))
    }
}

/// VariantAccess implementation.
impl<'de> de::VariantAccess<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        de::DeserializeSeed::deserialize(seed, self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self, fields.len(), visitor)
    }
}

/// Deserialize a value from bytes (without message header).
pub fn deserialize_data<'de, T>(data: &'de [u8]) -> Result<T>
where
    T: de::Deserialize<'de>,
{
    let mut deserializer = Deserializer::new(data);
    let value = T::deserialize(&mut deserializer)?;
    Ok(value)
}
