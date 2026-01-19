//! Request and Response packet definitions

use std::collections::HashMap;
use crate::{Result, codec::{Buffer, Reader}};

/// Request packet structure
#[derive(Debug, Clone, Default)]
pub struct RequestPacket {
    /// Protocol version (tag 1)
    pub i_version: i16,
    /// Packet type: 0=normal, 1=oneway (tag 2)
    pub c_packet_type: i8,
    /// Message type flags (tag 3)
    pub i_message_type: i32,
    /// Request ID (tag 4)
    pub i_request_id: i32,
    /// Servant name (tag 5)
    pub s_servant_name: String,
    /// Function name (tag 6)
    pub s_func_name: String,
    /// Request buffer (tag 7)
    pub s_buffer: Vec<u8>,
    /// Timeout in milliseconds (tag 8)
    pub i_timeout: i32,
    /// Context map (tag 9)
    pub context: HashMap<String, String>,
    /// Status map (tag 10)
    pub status: HashMap<String, String>,
}

impl RequestPacket {
    /// Create a new empty request packet
    pub fn new() -> Self {
        Self {
            i_version: crate::consts::TARS_VERSION,
            c_packet_type: crate::consts::TARS_NORMAL,
            i_message_type: 0,
            i_request_id: 0,
            s_servant_name: String::new(),
            s_func_name: String::new(),
            s_buffer: Vec::new(),
            i_timeout: crate::consts::DEFAULT_ASYNC_TIMEOUT as i32,
            context: HashMap::new(),
            status: HashMap::new(),
        }
    }

    /// Check if packet has a specific message type flag
    pub fn has_message_type(&self, msg_type: i32) -> bool {
        (self.i_message_type & msg_type) != 0
    }

    /// Add a message type flag
    pub fn add_message_type(&mut self, msg_type: i32) {
        self.i_message_type |= msg_type;
    }

    /// Check if this is a oneway request
    pub fn is_oneway(&self) -> bool {
        self.c_packet_type == crate::consts::TARS_ONEWAY
    }

    /// Encode to bytes with length prefix
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut buf = Buffer::with_capacity(256);
        self.write_to(&mut buf)?;
        Ok(buf.to_bytes_with_length())
    }

    /// Write packet to buffer (without length prefix)
    pub fn write_to(&self, buf: &mut Buffer) -> Result<()> {
        buf.write_int16(self.i_version, 1)?;
        buf.write_int8(self.c_packet_type, 2)?;
        buf.write_int32(self.i_message_type, 3)?;
        buf.write_int32(self.i_request_id, 4)?;
        buf.write_string(&self.s_servant_name, 5)?;
        buf.write_string(&self.s_func_name, 6)?;
        buf.write_bytes_as_int8_vec(&self.s_buffer, 7)?;
        buf.write_int32(self.i_timeout, 8)?;
        buf.write_string_map(&self.context, 9)?;
        buf.write_string_map(&self.status, 10)?;
        Ok(())
    }

    /// Decode from bytes (with or without length prefix)
    pub fn decode(data: &[u8]) -> Result<Self> {
        let data = if data.len() >= 4 {
            let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
            if len == data.len() {
                &data[4..]
            } else {
                data
            }
        } else {
            data
        };

        let mut reader = Reader::new(data);
        Self::read_from(&mut reader)
    }

    /// Read packet from reader
    pub fn read_from(reader: &mut Reader) -> Result<Self> {
        let mut packet = Self::new();
        packet.i_version = reader.read_int16(1, false)?;
        packet.c_packet_type = reader.read_int8(2, false)?;
        packet.i_message_type = reader.read_int32(3, false)?;
        packet.i_request_id = reader.read_int32(4, false)?;
        packet.s_servant_name = reader.read_string(5, false)?;
        packet.s_func_name = reader.read_string(6, false)?;
        packet.s_buffer = reader.read_bytes(7, false)?;
        packet.i_timeout = reader.read_int32(8, false)?;
        packet.context = reader.read_string_map(9, false)?;
        packet.status = reader.read_string_map(10, false)?;
        Ok(packet)
    }
}

/// Response packet structure
#[derive(Debug, Clone, Default)]
pub struct ResponsePacket {
    /// Protocol version (tag 1)
    pub i_version: i16,
    /// Packet type (tag 2)
    pub c_packet_type: i8,
    /// Request ID (tag 3)
    pub i_request_id: i32,
    /// Message type flags (tag 4)
    pub i_message_type: i32,
    /// Return code: 0=success (tag 5)
    pub i_ret: i32,
    /// Response buffer (tag 6)
    pub s_buffer: Vec<u8>,
    /// Status map (tag 7)
    pub status: HashMap<String, String>,
    /// Result description (tag 8)
    pub s_result_desc: String,
    /// Context map (tag 9)
    pub context: HashMap<String, String>,
}

impl ResponsePacket {
    /// Create a new empty response packet
    pub fn new() -> Self {
        Self {
            i_version: crate::consts::TARS_VERSION,
            c_packet_type: crate::consts::TARS_NORMAL,
            i_request_id: 0,
            i_message_type: 0,
            i_ret: 0,
            s_buffer: Vec::new(),
            status: HashMap::new(),
            s_result_desc: String::new(),
            context: HashMap::new(),
        }
    }

    /// Create a success response
    pub fn success(request_id: i32, buffer: Vec<u8>) -> Self {
        Self {
            i_version: crate::consts::TARS_VERSION,
            c_packet_type: crate::consts::TARS_NORMAL,
            i_request_id: request_id,
            i_message_type: 0,
            i_ret: crate::consts::TARS_SERVER_SUCCESS,
            s_buffer: buffer,
            status: HashMap::new(),
            s_result_desc: String::new(),
            context: HashMap::new(),
        }
    }

    /// Create an error response
    pub fn error(request_id: i32, ret: i32, desc: &str) -> Self {
        Self {
            i_version: crate::consts::TARS_VERSION,
            c_packet_type: crate::consts::TARS_NORMAL,
            i_request_id: request_id,
            i_message_type: 0,
            i_ret: ret,
            s_buffer: Vec::new(),
            status: HashMap::new(),
            s_result_desc: desc.to_string(),
            context: HashMap::new(),
        }
    }

    /// Create a timeout response
    pub fn timeout(request_id: i32) -> Self {
        Self::error(
            request_id,
            crate::consts::TARS_SERVER_QUEUE_TIMEOUT,
            "server invoke timeout",
        )
    }

    /// Check if response is successful
    pub fn is_success(&self) -> bool {
        self.i_ret == crate::consts::TARS_SERVER_SUCCESS
    }

    /// Encode to bytes with length prefix
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut buf = Buffer::with_capacity(256);
        self.write_to(&mut buf)?;
        Ok(buf.to_bytes_with_length())
    }

    /// Write packet to buffer (without length prefix)
    pub fn write_to(&self, buf: &mut Buffer) -> Result<()> {
        buf.write_int16(self.i_version, 1)?;
        buf.write_int8(self.c_packet_type, 2)?;
        buf.write_int32(self.i_request_id, 3)?;
        buf.write_int32(self.i_message_type, 4)?;
        buf.write_int32(self.i_ret, 5)?;
        buf.write_bytes_as_int8_vec(&self.s_buffer, 6)?;
        buf.write_string_map(&self.status, 7)?;
        buf.write_string(&self.s_result_desc, 8)?;
        buf.write_string_map(&self.context, 9)?;
        Ok(())
    }

    /// Decode from bytes (with or without length prefix)
    pub fn decode(data: &[u8]) -> Result<Self> {
        let data = if data.len() >= 4 {
            let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
            if len == data.len() {
                &data[4..]
            } else {
                data
            }
        } else {
            data
        };

        let mut reader = Reader::new(data);
        Self::read_from(&mut reader)
    }

    /// Read packet from reader
    pub fn read_from(reader: &mut Reader) -> Result<Self> {
        let mut packet = Self::new();
        packet.i_version = reader.read_int16(1, false)?;
        packet.c_packet_type = reader.read_int8(2, false)?;
        packet.i_request_id = reader.read_int32(3, false)?;
        packet.i_message_type = reader.read_int32(4, false)?;
        packet.i_ret = reader.read_int32(5, false)?;
        packet.s_buffer = reader.read_bytes(6, false)?;
        packet.status = reader.read_string_map(7, false)?;
        packet.s_result_desc = reader.read_string(8, false)?;
        packet.context = reader.read_string_map(9, false)?;
        Ok(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_packet_oneway() {
        let mut req = RequestPacket::new();
        assert!(!req.is_oneway());

        req.c_packet_type = crate::consts::TARS_ONEWAY;
        assert!(req.is_oneway());
    }

    #[test]
    fn test_request_packet_message_type() {
        let mut req = RequestPacket::new();
        assert!(!req.has_message_type(crate::consts::TARS_MESSAGE_TYPE_DYED));

        req.add_message_type(crate::consts::TARS_MESSAGE_TYPE_DYED);
        assert!(req.has_message_type(crate::consts::TARS_MESSAGE_TYPE_DYED));
        assert!(!req.has_message_type(crate::consts::TARS_MESSAGE_TYPE_TRACE));

        req.add_message_type(crate::consts::TARS_MESSAGE_TYPE_TRACE);
        assert!(req.has_message_type(crate::consts::TARS_MESSAGE_TYPE_DYED));
        assert!(req.has_message_type(crate::consts::TARS_MESSAGE_TYPE_TRACE));
    }

    #[test]
    fn test_response_packet_success() {
        let rsp = ResponsePacket::success(123, vec![1, 2, 3]);
        assert!(rsp.is_success());
        assert_eq!(rsp.i_request_id, 123);
        assert_eq!(rsp.s_buffer, vec![1, 2, 3]);
    }

    #[test]
    fn test_response_packet_error() {
        let rsp = ResponsePacket::error(123, -1, "error message");
        assert!(!rsp.is_success());
        assert_eq!(rsp.i_ret, -1);
        assert_eq!(rsp.s_result_desc, "error message");
    }

    #[test]
    fn test_response_packet_timeout() {
        let rsp = ResponsePacket::timeout(123);
        assert!(!rsp.is_success());
        assert_eq!(rsp.i_ret, crate::consts::TARS_SERVER_QUEUE_TIMEOUT);
    }
}
