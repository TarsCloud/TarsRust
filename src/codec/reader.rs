//! Reader for reading Tars encoded data

use std::io::{Cursor, Read};
use byteorder::{BigEndian, ReadBytesExt};
use crate::{Result, TarsError};
use super::types::{TarsType, Head};

/// Reader for reading Tars encoded data
pub struct Reader<'a> {
    /// Reference to the original data
    data: &'a [u8],
    /// Cursor for reading
    cursor: Cursor<&'a [u8]>,
}

impl<'a> Reader<'a> {
    /// Create a new reader from bytes
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            cursor: Cursor::new(data),
        }
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.cursor.position() as usize
    }

    /// Get remaining bytes
    pub fn remaining(&self) -> usize {
        self.data.len() - self.position()
    }

    /// Check if has more data
    pub fn has_more(&self) -> bool {
        self.remaining() > 0
    }

    /// Peek the next head without consuming
    pub fn peek_head(&self) -> Result<Head> {
        let pos = self.cursor.position();
        let mut temp_cursor = Cursor::new(self.data);
        temp_cursor.set_position(pos);
        Self::read_head_from_cursor(&mut temp_cursor)
    }

    /// Read a head (type and tag)
    /// IMPORTANT: The Tars protocol uses (tag << 4) | type format
    /// - High nibble (bits 4-7): tag (0-14, or 15 for extended tag)
    /// - Low nibble (bits 0-3): type
    fn read_head_from_cursor(cursor: &mut Cursor<&[u8]>) -> Result<Head> {
        let byte = cursor.read_u8().map_err(|_| TarsError::Codec("unexpected EOF reading head".into()))?;
        let ty = TarsType::try_from(byte & 0x0F)?;  // Low nibble is type
        let mut tag = (byte >> 4) & 0x0F;           // High nibble is tag

        if tag == 0x0F {
            // Extended tag: next byte is the actual tag value
            tag = cursor.read_u8().map_err(|_| TarsError::Codec("unexpected EOF reading extended tag".into()))?;
        }

        Ok(Head::new(ty, tag))
    }

    /// Read a head
    pub fn read_head(&mut self) -> Result<Head> {
        Self::read_head_from_cursor(&mut self.cursor)
    }

    /// Skip to a specific tag, returning the type if found
    fn skip_to_tag(&mut self, target_tag: u8) -> Result<Option<Head>> {
        while self.has_more() {
            let head = self.peek_head()?;

            if head.is_struct_end() || head.tag > target_tag {
                return Ok(None);
            }

            if head.tag == target_tag {
                // Consume the head
                self.read_head()?;
                return Ok(Some(head));
            }

            // Skip this field
            self.read_head()?;
            self.skip_field(&head)?;
        }
        Ok(None)
    }

    /// Skip a field based on its type
    fn skip_field(&mut self, head: &Head) -> Result<()> {
        match head.ty {
            TarsType::Byte => { self.cursor.read_u8()?; }
            TarsType::Short => { self.cursor.read_i16::<BigEndian>()?; }
            TarsType::Int => { self.cursor.read_i32::<BigEndian>()?; }
            TarsType::Long => { self.cursor.read_i64::<BigEndian>()?; }
            TarsType::Float => { self.cursor.read_f32::<BigEndian>()?; }
            TarsType::Double => { self.cursor.read_f64::<BigEndian>()?; }
            TarsType::String1 => {
                let len = self.cursor.read_u8()? as usize;
                self.cursor.set_position(self.cursor.position() + len as u64);
            }
            TarsType::String4 => {
                let len = self.cursor.read_u32::<BigEndian>()? as usize;
                self.cursor.set_position(self.cursor.position() + len as u64);
            }
            TarsType::Map => {
                let size = self.read_int32(0, true)?;
                for _ in 0..size {
                    // Skip key
                    let key_head = self.read_head()?;
                    self.skip_field(&key_head)?;
                    // Skip value
                    let val_head = self.read_head()?;
                    self.skip_field(&val_head)?;
                }
            }
            TarsType::List => {
                let size = self.read_int32(0, true)?;
                for _ in 0..size {
                    let item_head = self.read_head()?;
                    self.skip_field(&item_head)?;
                }
            }
            TarsType::StructBegin => {
                self.skip_to_struct_end()?;
            }
            TarsType::StructEnd => {}
            TarsType::ZeroTag => {}
            TarsType::SimpleList => {
                let _inner_head = self.read_head()?;
                let len = self.read_int32(0, true)? as usize;
                self.cursor.set_position(self.cursor.position() + len as u64);
            }
        }
        Ok(())
    }

    /// Skip to struct end marker
    pub fn skip_to_struct_end(&mut self) -> Result<()> {
        while self.has_more() {
            let head = self.read_head()?;
            if head.is_struct_end() {
                return Ok(());
            }
            self.skip_field(&head)?;
        }
        Err(TarsError::Codec("unexpected EOF before struct end".into()))
    }

    /// Read int8 value
    pub fn read_int8(&mut self, tag: u8, require: bool) -> Result<i8> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0)
                }
            }
            Some(head) => {
                match head.ty {
                    TarsType::ZeroTag => Ok(0),
                    TarsType::Byte => Ok(self.cursor.read_i8()?),
                    _ => Err(TarsError::Codec(format!("type mismatch for int8: {:?}", head.ty))),
                }
            }
        }
    }

    /// Read int16 value
    pub fn read_int16(&mut self, tag: u8, require: bool) -> Result<i16> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0)
                }
            }
            Some(head) => {
                match head.ty {
                    TarsType::ZeroTag => Ok(0),
                    TarsType::Byte => Ok(self.cursor.read_i8()? as i16),
                    TarsType::Short => Ok(self.cursor.read_i16::<BigEndian>()?),
                    _ => Err(TarsError::Codec(format!("type mismatch for int16: {:?}", head.ty))),
                }
            }
        }
    }

    /// Read int32 value
    pub fn read_int32(&mut self, tag: u8, require: bool) -> Result<i32> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0)
                }
            }
            Some(head) => {
                match head.ty {
                    TarsType::ZeroTag => Ok(0),
                    TarsType::Byte => Ok(self.cursor.read_i8()? as i32),
                    TarsType::Short => Ok(self.cursor.read_i16::<BigEndian>()? as i32),
                    TarsType::Int => Ok(self.cursor.read_i32::<BigEndian>()?),
                    _ => Err(TarsError::Codec(format!("type mismatch for int32: {:?}", head.ty))),
                }
            }
        }
    }

    /// Read int64 value
    pub fn read_int64(&mut self, tag: u8, require: bool) -> Result<i64> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0)
                }
            }
            Some(head) => {
                match head.ty {
                    TarsType::ZeroTag => Ok(0),
                    TarsType::Byte => Ok(self.cursor.read_i8()? as i64),
                    TarsType::Short => Ok(self.cursor.read_i16::<BigEndian>()? as i64),
                    TarsType::Int => Ok(self.cursor.read_i32::<BigEndian>()? as i64),
                    TarsType::Long => Ok(self.cursor.read_i64::<BigEndian>()?),
                    _ => Err(TarsError::Codec(format!("type mismatch for int64: {:?}", head.ty))),
                }
            }
        }
    }

    /// Read float value
    pub fn read_float(&mut self, tag: u8, require: bool) -> Result<f32> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0.0)
                }
            }
            Some(head) => {
                match head.ty {
                    TarsType::ZeroTag => Ok(0.0),
                    TarsType::Float => Ok(self.cursor.read_f32::<BigEndian>()?),
                    _ => Err(TarsError::Codec(format!("type mismatch for float: {:?}", head.ty))),
                }
            }
        }
    }

    /// Read double value
    pub fn read_double(&mut self, tag: u8, require: bool) -> Result<f64> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0.0)
                }
            }
            Some(head) => {
                match head.ty {
                    TarsType::ZeroTag => Ok(0.0),
                    TarsType::Float => Ok(self.cursor.read_f32::<BigEndian>()? as f64),
                    TarsType::Double => Ok(self.cursor.read_f64::<BigEndian>()?),
                    _ => Err(TarsError::Codec(format!("type mismatch for double: {:?}", head.ty))),
                }
            }
        }
    }

    /// Read bool value
    pub fn read_bool(&mut self, tag: u8, require: bool) -> Result<bool> {
        Ok(self.read_int8(tag, require)? != 0)
    }

    /// Read string value
    pub fn read_string(&mut self, tag: u8, require: bool) -> Result<String> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(String::new())
                }
            }
            Some(head) => {
                let len = match head.ty {
                    TarsType::String1 => self.cursor.read_u8()? as usize,
                    TarsType::String4 => self.cursor.read_u32::<BigEndian>()? as usize,
                    _ => return Err(TarsError::Codec(format!("type mismatch for string: {:?}", head.ty))),
                };

                let mut buf = vec![0u8; len];
                self.cursor.read_exact(&mut buf)?;
                String::from_utf8(buf).map_err(|e| TarsError::Codec(format!("invalid UTF-8: {}", e)))
            }
        }
    }

    /// Read bytes value
    pub fn read_bytes(&mut self, tag: u8, require: bool) -> Result<Vec<u8>> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(Vec::new())
                }
            }
            Some(head) => {
                match head.ty {
                    TarsType::SimpleList => {
                        // Read inner head (should be Byte)
                        let _inner_head = self.read_head()?;
                        let len = self.read_int32(0, true)? as usize;
                        let mut buf = vec![0u8; len];
                        self.cursor.read_exact(&mut buf)?;
                        Ok(buf)
                    }
                    TarsType::List => {
                        let len = self.read_int32(0, true)? as usize;
                        let mut buf = Vec::with_capacity(len);
                        for _ in 0..len {
                            let item_head = self.read_head()?;
                            match item_head.ty {
                                TarsType::ZeroTag => buf.push(0),
                                TarsType::Byte => buf.push(self.cursor.read_i8()? as u8),
                                _ => return Err(TarsError::Codec(format!("invalid byte array element: {:?}", item_head.ty))),
                            }
                        }
                        Ok(buf)
                    }
                    _ => Err(TarsError::Codec(format!("type mismatch for bytes: {:?}", head.ty))),
                }
            }
        }
    }

    /// Read a map size and prepare for reading key-value pairs
    pub fn read_map_begin(&mut self, tag: u8, require: bool) -> Result<i32> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0)
                }
            }
            Some(head) => {
                if head.ty != TarsType::Map {
                    return Err(TarsError::Codec(format!("type mismatch for map: {:?}", head.ty)));
                }
                self.read_int32(0, true)
            }
        }
    }

    /// Read a list size and prepare for reading elements
    pub fn read_list_begin(&mut self, tag: u8, require: bool) -> Result<i32> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(0)
                }
            }
            Some(head) => {
                if head.ty != TarsType::List {
                    return Err(TarsError::Codec(format!("type mismatch for list: {:?}", head.ty)));
                }
                self.read_int32(0, true)
            }
        }
    }

    /// Read struct begin
    pub fn read_struct_begin(&mut self, tag: u8, require: bool) -> Result<bool> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(false)
                }
            }
            Some(head) => {
                if head.ty != TarsType::StructBegin {
                    return Err(TarsError::Codec(format!("type mismatch for struct: {:?}", head.ty)));
                }
                Ok(true)
            }
        }
    }

    /// Read struct end
    pub fn read_struct_end(&mut self) -> Result<()> {
        self.skip_to_struct_end()
    }

    /// Skip to struct begin for the given tag
    pub fn skip_to_struct_begin(&mut self, tag: u8, require: bool) -> Result<bool> {
        self.read_struct_begin(tag, require)
    }

    /// Skip to list for the given tag
    pub fn skip_to_list(&mut self, tag: u8, require: bool) -> Result<bool> {
        match self.skip_to_tag(tag)? {
            None => {
                if require {
                    Err(TarsError::Codec(format!("required tag {} not found", tag)))
                } else {
                    Ok(false)
                }
            }
            Some(head) => {
                if head.ty != TarsType::List {
                    return Err(TarsError::Codec(format!("type mismatch for list: {:?}", head.ty)));
                }
                Ok(true)
            }
        }
    }

    /// Read a string->string map
    pub fn read_string_map(&mut self, tag: u8, require: bool) -> Result<std::collections::HashMap<String, String>> {
        let size = self.read_map_begin(tag, require)?;
        let mut map = std::collections::HashMap::with_capacity(size as usize);
        for _ in 0..size {
            let key = self.read_string(0, true)?;
            let value = self.read_string(1, true)?;
            map.insert(key, value);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Buffer;

    #[test]
    fn test_read_int() {
        let mut buf = Buffer::new();
        buf.write_int8(10, 0).unwrap();
        buf.write_int16(1000, 1).unwrap();
        buf.write_int32(100000, 2).unwrap();
        buf.write_int64(10000000000i64, 3).unwrap();

        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_int8(0, true).unwrap(), 10);
        assert_eq!(reader.read_int16(1, true).unwrap(), 1000);
        assert_eq!(reader.read_int32(2, true).unwrap(), 100000);
        assert_eq!(reader.read_int64(3, true).unwrap(), 10000000000i64);
    }

    #[test]
    fn test_read_optional() {
        let mut buf = Buffer::new();
        buf.write_int32(100, 0).unwrap();

        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);

        // Required tag exists
        assert_eq!(reader.read_int32(0, true).unwrap(), 100);

        // Optional tag doesn't exist
        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);
        assert_eq!(reader.read_int32(0, true).unwrap(), 100);
        assert_eq!(reader.read_int32(1, false).unwrap(), 0);
    }

    #[test]
    fn test_read_string() {
        let mut buf = Buffer::new();
        buf.write_string("hello world", 0).unwrap();

        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_string(0, true).unwrap(), "hello world");
    }

    #[test]
    fn test_read_bytes() {
        let mut buf = Buffer::new();
        buf.write_bytes(&[1, 2, 3, 4, 5], 0).unwrap();

        let data = buf.to_bytes();
        let mut reader = Reader::new(&data);

        assert_eq!(reader.read_bytes(0, true).unwrap(), vec![1, 2, 3, 4, 5]);
    }
}
