//! Serializing Rust data types into the Blueberry wire format.

use byteorder::{ByteOrder, LittleEndian};
use serde::ser;

use crate::error::{Error, Result};

/// A serializer that writes values into a `Vec<u8>` buffer using the Blueberry
/// wire format.
///
/// The buffer approach (rather than streaming `Write`) is required because
/// sequence data blocks are deferred to the end of the message and sequence
/// headers need index fixups.
pub struct Serializer {
    /// Main message body buffer.
    buf: Vec<u8>,
    /// Current write position (used for alignment calculations).
    pos: usize,
    /// Deferred sequence data blocks, appended after the main body.
    seq_data_blocks: Vec<Vec<u8>>,
    /// Fixups: (offset of sequence header in buf, index into seq_data_blocks).
    seq_fixups: Vec<(usize, usize)>,
    /// Current bit position for boolean packing (0..8).
    bool_bit_pos: u8,
    /// Offset of the current boolean packing byte in `buf`, if active.
    bool_byte_offset: Option<usize>,
    /// Whether we are currently writing inside a sequence data block
    /// (suppresses alignment padding for struct fields).
    in_seq_data: bool,
    /// Number of top-level struct fields serialized (for max_ordinal calculation).
    field_count: usize,
    /// Base offset added to all sequence indices during finalize.
    /// Set to HEADER_SIZE when serializing a message so sequence indices
    /// are relative to the message start (including header), not just the body.
    base_offset: usize,
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer {
    /// Create a new serializer.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            pos: 0,
            seq_data_blocks: Vec::new(),
            seq_fixups: Vec::new(),
            bool_bit_pos: 0,
            bool_byte_offset: None,
            in_seq_data: false,
            field_count: 0,
            base_offset: 0,
        }
    }

    /// Create a new serializer with the given pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            pos: 0,
            seq_data_blocks: Vec::new(),
            seq_fixups: Vec::new(),
            bool_bit_pos: 0,
            bool_byte_offset: None,
            in_seq_data: false,
            field_count: 0,
            base_offset: 0,
        }
    }

    /// Set a base offset that will be added to all sequence indices in
    /// `finalize()`. Used when the body will be preceded by a header.
    pub fn set_base_offset(&mut self, offset: usize) {
        self.base_offset = offset;
    }

    /// Consume the serializer and return the final encoded bytes.
    ///
    /// This appends all deferred sequence data blocks to the body and fixes up
    /// the sequence header index fields.
    pub fn finalize(mut self) -> Vec<u8> {
        self.flush_bools();
        let body_len = self.buf.len();

        // Append each sequence data block and fix up the header.
        // base_offset is added so indices are relative to the message start
        // (e.g. when a header is prepended).
        let mut data_offset = self.base_offset + body_len;
        for (header_offset, block_idx) in &self.seq_fixups {
            let block = &self.seq_data_blocks[*block_idx];
            let index = data_offset as u16;
            LittleEndian::write_u16(&mut self.buf[*header_offset..*header_offset + 2], index);
            data_offset += block.len();
        }

        // Actually append the data blocks
        for block in &self.seq_data_blocks {
            self.buf.extend_from_slice(block);
        }

        self.buf
    }

    /// Returns the number of top-level fields serialized so far.
    pub fn field_count(&self) -> usize {
        self.field_count
    }

    /// Write alignment padding for a value of the given size.
    ///
    /// 8-byte types are aligned on 4-byte boundaries (not 8).
    /// When inside a sequence data block, no padding is written.
    fn write_padding(&mut self, size: usize) {
        if self.in_seq_data || size <= 1 {
            return;
        }
        // 8-byte types align on 4-byte boundary
        let align = if size >= 8 { 4 } else { size };
        let rem = self.pos % align;
        if rem != 0 {
            let pad = align - rem;
            self.buf.resize(self.buf.len() + pad, 0);
            self.pos += pad;
        }
    }

    /// Flush any pending boolean packing byte.
    fn flush_bools(&mut self) {
        self.bool_bit_pos = 0;
        self.bool_byte_offset = None;
    }

    /// Write a single byte.
    fn write_u8(&mut self, v: u8) {
        self.flush_bools();
        self.write_padding(1);
        self.buf.push(v);
        self.pos += 1;
    }

    /// Write a signed byte.
    fn write_i8(&mut self, v: i8) {
        self.write_u8(v as u8);
    }

    /// Write a u16 in little-endian.
    fn write_u16(&mut self, v: u16) {
        self.flush_bools();
        self.write_padding(2);
        let mut tmp = [0u8; 2];
        LittleEndian::write_u16(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 2;
    }

    /// Write an i16 in little-endian.
    fn write_i16(&mut self, v: i16) {
        self.flush_bools();
        self.write_padding(2);
        let mut tmp = [0u8; 2];
        LittleEndian::write_i16(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 2;
    }

    /// Write a u32 in little-endian.
    fn write_u32(&mut self, v: u32) {
        self.flush_bools();
        self.write_padding(4);
        let mut tmp = [0u8; 4];
        LittleEndian::write_u32(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 4;
    }

    /// Write an i32 in little-endian.
    fn write_i32(&mut self, v: i32) {
        self.flush_bools();
        self.write_padding(4);
        let mut tmp = [0u8; 4];
        LittleEndian::write_i32(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 4;
    }

    /// Write a u64 in little-endian (4-byte aligned, not 8).
    fn write_u64(&mut self, v: u64) {
        self.flush_bools();
        self.write_padding(8); // will use 4-byte alignment
        let mut tmp = [0u8; 8];
        LittleEndian::write_u64(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 8;
    }

    /// Write an i64 in little-endian (4-byte aligned, not 8).
    fn write_i64(&mut self, v: i64) {
        self.flush_bools();
        self.write_padding(8);
        let mut tmp = [0u8; 8];
        LittleEndian::write_i64(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 8;
    }

    /// Write an f32 in little-endian.
    fn write_f32(&mut self, v: f32) {
        self.flush_bools();
        self.write_padding(4);
        let mut tmp = [0u8; 4];
        LittleEndian::write_f32(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 4;
    }

    /// Write an f64 in little-endian (4-byte aligned, not 8).
    fn write_f64(&mut self, v: f64) {
        self.flush_bools();
        self.write_padding(8);
        let mut tmp = [0u8; 8];
        LittleEndian::write_f64(&mut tmp, v);
        self.buf.extend_from_slice(&tmp);
        self.pos += 8;
    }

    /// Write a boolean, packing consecutive bools into shared bytes (LSb to MSb).
    fn write_bool(&mut self, v: bool) {
        if let Some(offset) = self.bool_byte_offset {
            // Pack into existing byte
            if v {
                self.buf[offset] |= 1 << self.bool_bit_pos;
            }
            self.bool_bit_pos += 1;
            if self.bool_bit_pos >= 8 {
                self.bool_bit_pos = 0;
                self.bool_byte_offset = None;
            }
        } else {
            // Start a new packing byte
            self.write_padding(1);
            let offset = self.buf.len();
            self.buf.push(if v { 1 } else { 0 });
            self.pos += 1;
            self.bool_bit_pos = 1;
            self.bool_byte_offset = Some(offset);
        }
    }

    #[allow(dead_code)]
    fn write_usize_as_u32(&mut self, v: usize) -> Result<()> {
        if v > u32::MAX as usize {
            return Err(Error::NumberOutOfRange);
        }
        self.write_u32(v as u32);
        Ok(())
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SequenceSerializer<'a>;
    type SerializeTuple = StructCompound<'a>;
    type SerializeTupleStruct = StructCompound<'a>;
    type SerializeTupleVariant = StructCompound<'a>;
    type SerializeMap = StructCompound<'a>;
    type SerializeStruct = StructCompound<'a>;
    type SerializeStructVariant = StructCompound<'a>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.write_bool(v);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.write_i8(v);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.write_i16(v);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.write_i32(v);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write_i64(v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.write_u8(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.write_u16(v);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.write_u32(v);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.write_u64(v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.write_f32(v);
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.write_f64(v);
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        let mut buf = [0u8; 4];
        let s = v.encode_utf8(&mut buf);
        self.serialize_str(s)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        // Strings are sequences of UTF-8 bytes.
        // Write sequence header inline, data deferred to end.
        let bytes = v.as_bytes();
        self.flush_bools();

        // Sequence header: u16 index (placeholder) + u16 elementByteLength (1 for u8)
        self.write_padding(2);
        let header_offset = self.buf.len();
        self.buf.extend_from_slice(&[0u8; 4]); // placeholder
        self.pos += 4;

        // elementByteLength = 1 (UTF-8 bytes)
        LittleEndian::write_u16(&mut self.buf[header_offset + 2..header_offset + 4], 1);

        // Build the data block: u32 length + bytes
        let block_idx = self.seq_data_blocks.len();
        let mut block = Vec::with_capacity(4 + bytes.len());
        let mut tmp = [0u8; 4];
        LittleEndian::write_u32(&mut tmp, bytes.len() as u32);
        block.extend_from_slice(&tmp);
        block.extend_from_slice(bytes);
        self.seq_data_blocks.push(block);

        self.seq_fixups.push((header_offset, block_idx));
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        // Treat as a sequence of u8
        self.flush_bools();

        self.write_padding(2);
        let header_offset = self.buf.len();
        self.buf.extend_from_slice(&[0u8; 4]);
        self.pos += 4;

        LittleEndian::write_u16(&mut self.buf[header_offset + 2..header_offset + 4], 1);

        let block_idx = self.seq_data_blocks.len();
        let mut block = Vec::with_capacity(4 + v.len());
        let mut tmp = [0u8; 4];
        LittleEndian::write_u32(&mut tmp, v.len() as u32);
        block.extend_from_slice(&tmp);
        block.extend_from_slice(v);
        self.seq_data_blocks.push(block);

        self.seq_fixups.push((header_offset, block_idx));
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        Err(Error::TypeNotSupported)
    }

    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        Err(Error::TypeNotSupported)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.serialize_u32(variant_index)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        self.serialize_u32(variant_index)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let _len = len.ok_or(Error::SequenceMustHaveLength)?;
        self.flush_bools();

        // Write sequence header in main body: u16 index + u16 elementByteLength
        self.write_padding(2);
        let header_offset = self.buf.len();
        self.buf.extend_from_slice(&[0u8; 4]); // placeholder
        self.pos += 4;

        let block_idx = self.seq_data_blocks.len();
        self.seq_data_blocks.push(Vec::new());
        self.seq_fixups.push((header_offset, block_idx));

        Ok(SequenceSerializer {
            ser: self,
            block_idx,
            header_offset,
            element_count: 0,
            first_element_size: None,
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.flush_bools();
        Ok(StructCompound { ser: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.flush_bools();
        Ok(StructCompound { ser: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.flush_bools();
        self.serialize_u32(variant_index)?;
        Ok(StructCompound { ser: self })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::TypeNotSupported)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.flush_bools();
        Ok(StructCompound { ser: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.flush_bools();
        self.serialize_u32(variant_index)?;
        Ok(StructCompound { ser: self })
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Helper for serializing struct/tuple compound types.
///
/// Fields are serialized inline into the main body buffer.
#[doc(hidden)]
pub struct StructCompound<'a> {
    ser: &'a mut Serializer,
}

impl<'a> ser::SerializeStruct for StructCompound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        self.ser.field_count += 1;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.flush_bools();
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for StructCompound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.flush_bools();
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for StructCompound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.flush_bools();
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for StructCompound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.flush_bools();
        Ok(())
    }
}

impl<'a> ser::SerializeMap for StructCompound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.flush_bools();
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for StructCompound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.flush_bools();
        Ok(())
    }
}

/// Helper for serializing sequences.
///
/// Sequence data (element count + elements) is written to a deferred data
/// block. A 4-byte header (u16 index + u16 elementByteLength) is placed inline
/// in the main body.
#[doc(hidden)]
pub struct SequenceSerializer<'a> {
    ser: &'a mut Serializer,
    block_idx: usize,
    header_offset: usize,
    element_count: usize,
    first_element_size: Option<usize>,
}

impl<'a> SequenceSerializer<'a> {
    /// Create a sub-serializer that writes into the data block.
    fn serialize_element_to_block<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        let block = &mut self.ser.seq_data_blocks[self.block_idx];
        let start_len = block.len();

        // Serialize element into a temporary serializer with in_seq_data=true
        let mut elem_ser = Serializer {
            buf: std::mem::take(block),
            pos: start_len, // continue from where the block left off
            seq_data_blocks: Vec::new(),
            seq_fixups: Vec::new(),
            bool_bit_pos: 0,
            bool_byte_offset: None,
            in_seq_data: true,
            field_count: 0,
            base_offset: 0,
        };
        value.serialize(&mut elem_ser)?;

        // Put the buffer back
        let result_buf = elem_ser.finalize();
        self.ser.seq_data_blocks[self.block_idx] = result_buf;

        let elem_size = self.ser.seq_data_blocks[self.block_idx].len() - start_len;
        if self.first_element_size.is_none() {
            self.first_element_size = Some(elem_size);
        }

        self.element_count += 1;
        Ok(())
    }
}

impl<'a> ser::SerializeSeq for SequenceSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize + ?Sized,
    {
        self.serialize_element_to_block(value)
    }

    fn end(self) -> Result<()> {
        // Prepend the u32 element count to the data block
        let block = &mut self.ser.seq_data_blocks[self.block_idx];
        let mut count_buf = [0u8; 4];
        LittleEndian::write_u32(&mut count_buf, self.element_count as u32);

        // Insert count at the beginning of the block
        let mut new_block = Vec::with_capacity(4 + block.len());
        new_block.extend_from_slice(&count_buf);
        new_block.append(block);
        *block = new_block;

        // Set the elementByteLength in the header
        let elem_byte_len = self.first_element_size.unwrap_or(0) as u16;
        LittleEndian::write_u16(
            &mut self.ser.buf[self.header_offset + 2..self.header_offset + 4],
            elem_byte_len,
        );

        Ok(())
    }
}

/// Serialize a value to bytes (without message header).
pub fn serialize_data<T>(value: &T) -> Result<Vec<u8>>
where
    T: ser::Serialize + ?Sized,
{
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.finalize())
}
