//! Buffer for writing Tars encoded data

use bytes::{BufMut, BytesMut};
use crate::Result;
use super::types::TarsType;

/// Buffer for writing Tars encoded data
#[derive(Debug)]
pub struct Buffer {
    buf: BytesMut,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            buf: BytesMut::new(),
        }
    }

    /// Create a new buffer with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(capacity),
        }
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }

    /// Get the current length
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.buf.to_vec()
    }

    /// Get a reference to the underlying bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Write raw bytes
    pub fn write_raw(&mut self, data: &[u8]) -> Result<()> {
        self.buf.extend_from_slice(data);
        Ok(())
    }

    /// Write TLV head
    /// IMPORTANT: The Tars protocol uses (tag << 4) | type, NOT (type << 4) | tag
    pub fn write_head(&mut self, ty: TarsType, tag: u8) -> Result<()> {
        if tag < 15 {
            // Single byte head: high 4 bits = tag, low 4 bits = type
            self.buf.put_u8((tag << 4) | ty.as_u8());
        } else {
            // Two byte head: first byte = 15 << 4 | type, second byte = tag
            self.buf.put_u8((15 << 4) | ty.as_u8());
            self.buf.put_u8(tag);
        }
        Ok(())
    }

    /// Write int8 value
    pub fn write_int8(&mut self, data: i8, tag: u8) -> Result<()> {
        if data == 0 {
            return self.write_head(TarsType::ZeroTag, tag);
        }
        self.write_head(TarsType::Byte, tag)?;
        self.buf.put_i8(data);
        Ok(())
    }

    /// Write int16 value (optimized storage)
    pub fn write_int16(&mut self, data: i16, tag: u8) -> Result<()> {
        if data >= i8::MIN as i16 && data <= i8::MAX as i16 {
            return self.write_int8(data as i8, tag);
        }
        self.write_head(TarsType::Short, tag)?;
        self.buf.put_i16(data);
        Ok(())
    }

    /// Write int32 value (optimized storage)
    pub fn write_int32(&mut self, data: i32, tag: u8) -> Result<()> {
        if data >= i16::MIN as i32 && data <= i16::MAX as i32 {
            return self.write_int16(data as i16, tag);
        }
        self.write_head(TarsType::Int, tag)?;
        self.buf.put_i32(data);
        Ok(())
    }

    /// Write int64 value (optimized storage)
    pub fn write_int64(&mut self, data: i64, tag: u8) -> Result<()> {
        if data >= i32::MIN as i64 && data <= i32::MAX as i64 {
            return self.write_int32(data as i32, tag);
        }
        self.write_head(TarsType::Long, tag)?;
        self.buf.put_i64(data);
        Ok(())
    }

    /// Write uint8 value
    pub fn write_uint8(&mut self, data: u8, tag: u8) -> Result<()> {
        self.write_int16(data as i16, tag)
    }

    /// Write uint16 value
    pub fn write_uint16(&mut self, data: u16, tag: u8) -> Result<()> {
        self.write_int32(data as i32, tag)
    }

    /// Write uint32 value
    pub fn write_uint32(&mut self, data: u32, tag: u8) -> Result<()> {
        self.write_int64(data as i64, tag)
    }

    /// Write float value
    pub fn write_float(&mut self, data: f32, tag: u8) -> Result<()> {
        if data == 0.0 {
            return self.write_head(TarsType::ZeroTag, tag);
        }
        self.write_head(TarsType::Float, tag)?;
        self.buf.put_f32(data);
        Ok(())
    }

    /// Write double value
    pub fn write_double(&mut self, data: f64, tag: u8) -> Result<()> {
        if data == 0.0 {
            return self.write_head(TarsType::ZeroTag, tag);
        }
        self.write_head(TarsType::Double, tag)?;
        self.buf.put_f64(data);
        Ok(())
    }

    /// Write bool value
    pub fn write_bool(&mut self, data: bool, tag: u8) -> Result<()> {
        self.write_int8(if data { 1 } else { 0 }, tag)
    }

    /// Write string value
    pub fn write_string(&mut self, data: &str, tag: u8) -> Result<()> {
        let bytes = data.as_bytes();
        if bytes.len() > 255 {
            self.write_head(TarsType::String4, tag)?;
            self.buf.put_u32(bytes.len() as u32);
        } else {
            self.write_head(TarsType::String1, tag)?;
            self.buf.put_u8(bytes.len() as u8);
        }
        self.buf.extend_from_slice(bytes);
        Ok(())
    }

    /// Write bytes value (SimpleList)
    pub fn write_bytes(&mut self, data: &[u8], tag: u8) -> Result<()> {
        self.write_head(TarsType::SimpleList, tag)?;
        self.write_head(TarsType::Byte, 0)?;
        self.write_int32(data.len() as i32, 0)?;
        self.buf.extend_from_slice(data);
        Ok(())
    }

    /// Write bytes as vector of int8 (for compatibility with Go version)
    pub fn write_bytes_as_int8_vec(&mut self, data: &[u8], tag: u8) -> Result<()> {
        self.write_head(TarsType::SimpleList, tag)?;
        self.write_head(TarsType::Byte, 0)?;
        self.write_int32(data.len() as i32, 0)?;
        self.buf.extend_from_slice(data);
        Ok(())
    }

    /// Write map header
    pub fn write_map(&mut self, size: usize, tag: u8) -> Result<()> {
        self.write_head(TarsType::Map, tag)?;
        self.write_int32(size as i32, 0)
    }

    /// Write list header
    pub fn write_list(&mut self, size: usize, tag: u8) -> Result<()> {
        self.write_head(TarsType::List, tag)?;
        self.write_int32(size as i32, 0)
    }

    /// Write struct begin marker
    pub fn write_struct_begin(&mut self, tag: u8) -> Result<()> {
        self.write_head(TarsType::StructBegin, tag)
    }

    /// Write struct end marker
    pub fn write_struct_end(&mut self) -> Result<()> {
        self.write_head(TarsType::StructEnd, 0)
    }

    /// Write a string->string map
    pub fn write_string_map(&mut self, data: &std::collections::HashMap<String, String>, tag: u8) -> Result<()> {
        self.write_map(data.len(), tag)?;
        for (k, v) in data {
            self.write_string(k, 0)?;
            self.write_string(v, 1)?;
        }
        Ok(())
    }

    /// Convert buffer to bytes with 4-byte length prefix (big endian)
    pub fn to_bytes_with_length(&self) -> Vec<u8> {
        let data = self.as_bytes();
        let total_len = data.len() + 4;
        let mut result = Vec::with_capacity(total_len);
        result.extend_from_slice(&(total_len as u32).to_be_bytes());
        result.extend_from_slice(data);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_head_small_tag() {
        let mut buf = Buffer::new();
        buf.write_head(TarsType::Int, 5).unwrap();
        let bytes = buf.to_bytes();
        assert_eq!(bytes.len(), 1);
        // Tars format: (tag << 4) | type
        assert_eq!(bytes[0], (5 << 4) | TarsType::Int.as_u8());
    }

    #[test]
    fn test_write_head_large_tag() {
        let mut buf = Buffer::new();
        buf.write_head(TarsType::Int, 20).unwrap();
        let bytes = buf.to_bytes();
        assert_eq!(bytes.len(), 2);
        // Tars format: (15 << 4) | type, then tag byte
        assert_eq!(bytes[0], (15 << 4) | TarsType::Int.as_u8());
        assert_eq!(bytes[1], 20);
    }

    #[test]
    fn test_write_int_optimization() {
        // Zero should use ZeroTag
        let mut buf = Buffer::new();
        buf.write_int32(0, 0).unwrap();
        let bytes = buf.to_bytes();
        // Tars format: type in low nibble
        assert_eq!(bytes[0] & 0x0F, TarsType::ZeroTag.as_u8());

        // Small value should use Byte
        let mut buf = Buffer::new();
        buf.write_int32(100, 0).unwrap();
        let bytes = buf.to_bytes();
        assert_eq!(bytes[0] & 0x0F, TarsType::Byte.as_u8());

        // Large value should use Int
        let mut buf = Buffer::new();
        buf.write_int32(100000, 0).unwrap();
        let bytes = buf.to_bytes();
        assert_eq!(bytes[0] & 0x0F, TarsType::Int.as_u8());
    }

    #[test]
    fn test_write_string() {
        // Short string (< 256)
        let mut buf = Buffer::new();
        buf.write_string("hello", 0).unwrap();
        let bytes = buf.to_bytes();
        // Tars format: type in low nibble
        assert_eq!(bytes[0] & 0x0F, TarsType::String1.as_u8());
        assert_eq!(bytes[1], 5); // length

        // Long string (>= 256)
        let long_str = "x".repeat(300);
        let mut buf = Buffer::new();
        buf.write_string(&long_str, 0).unwrap();
        let bytes = buf.to_bytes();
        assert_eq!(bytes[0] & 0x0F, TarsType::String4.as_u8());
    }

    #[test]
    fn test_to_bytes_with_length() {
        let mut buf = Buffer::new();
        buf.write_int32(123, 0).unwrap();
        let bytes = buf.to_bytes_with_length();

        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        assert_eq!(len, bytes.len());
    }
}
