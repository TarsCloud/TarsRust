//! # Codec Module
//!
//! This module implements Tars protocol serialization and deserialization using TLV (Type-Length-Value) encoding.
//!
//! ## TLV Format
//!
//! The Tars protocol uses a compact binary format:
//! - High 4 bits: type (data type)
//! - Low 4 bits: tag (field identifier)
//! - If tag >= 15, an additional byte is used for the actual tag value
//!
//! ## Supported Types
//!
//! | Type | Value | Description |
//! |------|-------|-------------|
//! | BYTE | 0 | int8 |
//! | SHORT | 1 | int16 |
//! | INT | 2 | int32 |
//! | LONG | 3 | int64 |
//! | FLOAT | 4 | float32 |
//! | DOUBLE | 5 | float64 |
//! | STRING1 | 6 | Short string (length < 256) |
//! | STRING4 | 7 | Long string |
//! | MAP | 8 | Map type |
//! | LIST | 9 | List/Vector |
//! | STRUCT_BEGIN | 10 | Struct start marker |
//! | STRUCT_END | 11 | Struct end marker |
//! | ZERO_TAG | 12 | Zero value marker |
//! | SIMPLE_LIST | 13 | Simple list (bytes) |

mod buffer;
mod reader;
mod types;

pub use buffer::Buffer;
pub use reader::Reader;
pub use types::*;

/// Trait for types that can be serialized to Tars format
pub trait TarsEncode {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()>;
}

/// Trait for types that can be deserialized from Tars format
pub trait TarsDecode: Sized {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self>;
}

/// Trait for struct types with WriteTo/ReadFrom methods
pub trait TarsStruct: TarsEncode + TarsDecode {
    fn write_to(&self, buf: &mut Buffer) -> crate::Result<()>;
    fn read_from(reader: &mut Reader) -> crate::Result<Self>;
}

// Implement TarsEncode for primitive types
impl TarsEncode for i8 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_int8(*self, tag)
    }
}

impl TarsEncode for i16 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_int16(*self, tag)
    }
}

impl TarsEncode for i32 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_int32(*self, tag)
    }
}

impl TarsEncode for i64 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_int64(*self, tag)
    }
}

impl TarsEncode for u8 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_int8(*self as i8, tag)
    }
}

impl TarsEncode for u16 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_int16(*self as i16, tag)
    }
}

impl TarsEncode for u32 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_int32(*self as i32, tag)
    }
}

impl TarsEncode for f32 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_float(*self, tag)
    }
}

impl TarsEncode for f64 {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_double(*self, tag)
    }
}

impl TarsEncode for bool {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_bool(*self, tag)
    }
}

impl TarsEncode for String {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_string(self, tag)
    }
}

impl TarsEncode for &str {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_string(self, tag)
    }
}

impl TarsEncode for Vec<u8> {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_bytes(self, tag)
    }
}

impl TarsEncode for Vec<i32> {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_head(TarsType::List, tag)?;
        buf.write_int32(self.len() as i32, 0)?;
        for item in self {
            item.encode(buf, 0)?;
        }
        Ok(())
    }
}

impl TarsEncode for Vec<i64> {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_head(TarsType::List, tag)?;
        buf.write_int32(self.len() as i32, 0)?;
        for item in self {
            item.encode(buf, 0)?;
        }
        Ok(())
    }
}

impl TarsEncode for Vec<String> {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_head(TarsType::List, tag)?;
        buf.write_int32(self.len() as i32, 0)?;
        for item in self {
            item.encode(buf, 0)?;
        }
        Ok(())
    }
}

impl<K: TarsEncode, V: TarsEncode> TarsEncode for std::collections::HashMap<K, V> {
    fn encode(&self, buf: &mut Buffer, tag: u8) -> crate::Result<()> {
        buf.write_head(TarsType::Map, tag)?;
        buf.write_int32(self.len() as i32, 0)?;
        for (k, v) in self {
            k.encode(buf, 0)?;
            v.encode(buf, 1)?;
        }
        Ok(())
    }
}

// Implement TarsDecode for primitive types
impl TarsDecode for i8 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_int8(tag, require)
    }
}

impl TarsDecode for i16 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_int16(tag, require)
    }
}

impl TarsDecode for i32 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_int32(tag, require)
    }
}

impl TarsDecode for i64 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_int64(tag, require)
    }
}

impl TarsDecode for u8 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        Ok(reader.read_int8(tag, require)? as u8)
    }
}

impl TarsDecode for u16 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        Ok(reader.read_int16(tag, require)? as u16)
    }
}

impl TarsDecode for u32 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        Ok(reader.read_int32(tag, require)? as u32)
    }
}

impl TarsDecode for f32 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_float(tag, require)
    }
}

impl TarsDecode for f64 {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_double(tag, require)
    }
}

impl TarsDecode for bool {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_bool(tag, require)
    }
}

impl TarsDecode for String {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_string(tag, require)
    }
}

impl TarsDecode for Vec<u8> {
    fn decode(reader: &mut Reader, tag: u8, require: bool) -> crate::Result<Self> {
        reader.read_bytes(tag, require)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_int() {
        let mut buf = Buffer::new();
        buf.write_int32(12345, 0).unwrap();
        buf.write_int64(-9876543210, 1).unwrap();

        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);

        let v1: i32 = reader.read_int32(0, true).unwrap();
        let v2: i64 = reader.read_int64(1, true).unwrap();

        assert_eq!(v1, 12345);
        assert_eq!(v2, -9876543210);
    }

    #[test]
    fn test_encode_decode_string() {
        let mut buf = Buffer::new();
        buf.write_string("hello", 0).unwrap();
        buf.write_string("world with longer text that exceeds 255 characters... ".repeat(10).as_str(), 1).unwrap();

        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);

        let s1: String = reader.read_string(0, true).unwrap();
        let s2: String = reader.read_string(1, true).unwrap();

        assert_eq!(s1, "hello");
        assert!(s2.starts_with("world with longer"));
    }

    #[test]
    fn test_zero_value() {
        let mut buf = Buffer::new();
        buf.write_int32(0, 0).unwrap();
        buf.write_int64(0, 1).unwrap();

        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);

        let v1: i32 = reader.read_int32(0, true).unwrap();
        let v2: i64 = reader.read_int64(1, true).unwrap();

        assert_eq!(v1, 0);
        assert_eq!(v2, 0);
    }
}
