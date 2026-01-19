//! QueryF protocol types for service discovery
//!
//! Corresponds to QueryF.tars in TarsGo

use crate::codec::{Buffer, Reader};
use crate::Result;

/// EndpointF structure for service endpoint
#[derive(Debug, Clone, Default)]
pub struct EndpointF {
    pub host: String,
    pub port: i32,
    pub timeout: i32,
    pub istcp: i32,      // 0=UDP, 1=TCP, 2=SSL
    pub grid: i32,
    pub groupworkid: i32,
    pub grouprealid: i32,
    pub set_id: String,
    pub qos: i32,
    pub bak_flag: i32,
    pub weight: i32,
    pub weight_type: i32, // 0=round-robin, 1=static weight
    pub auth_type: i32,
}

impl EndpointF {
    pub fn encode(&self, buf: &mut Buffer) -> Result<()> {
        buf.write_string(&self.host, 0)?;
        buf.write_int32(self.port, 1)?;
        buf.write_int32(self.timeout, 2)?;
        buf.write_int32(self.istcp, 3)?;
        buf.write_int32(self.grid, 4)?;
        buf.write_int32(self.groupworkid, 5)?;
        buf.write_int32(self.grouprealid, 6)?;
        buf.write_string(&self.set_id, 7)?;
        buf.write_int32(self.qos, 8)?;
        buf.write_int32(self.bak_flag, 9)?;
        buf.write_int32(self.weight, 11)?;
        buf.write_int32(self.weight_type, 12)?;
        buf.write_int32(self.auth_type, 13)?;
        Ok(())
    }

    pub fn decode(reader: &mut Reader) -> Result<Self> {
        let mut ep = EndpointF::default();
        ep.host = reader.read_string(0, true)?;
        ep.port = reader.read_int32(1, true)?;
        ep.timeout = reader.read_int32(2, true)?;
        ep.istcp = reader.read_int32(3, true)?;
        ep.grid = reader.read_int32(4, true)?;
        ep.groupworkid = reader.read_int32(5, false).unwrap_or(0);
        ep.grouprealid = reader.read_int32(6, false).unwrap_or(0);
        ep.set_id = reader.read_string(7, false).unwrap_or_default();
        ep.qos = reader.read_int32(8, false).unwrap_or(0);
        ep.bak_flag = reader.read_int32(9, false).unwrap_or(0);
        ep.weight = reader.read_int32(11, false).unwrap_or(0);
        ep.weight_type = reader.read_int32(12, false).unwrap_or(0);
        ep.auth_type = reader.read_int32(13, false).unwrap_or(0);
        Ok(ep)
    }

    pub fn decode_from_struct(reader: &mut Reader, tag: u8, require: bool) -> Result<Self> {
        if !reader.skip_to_struct_begin(tag, require)? {
            return Ok(EndpointF::default());
        }
        let ep = Self::decode(reader)?;
        reader.skip_to_struct_end()?;
        Ok(ep)
    }
}

/// Decode a vector of EndpointF from reader
pub fn decode_endpoint_list(reader: &mut Reader, tag: u8, require: bool) -> Result<Vec<EndpointF>> {
    let mut result = Vec::new();

    if !reader.skip_to_list(tag, require)? {
        return Ok(result);
    }

    let count = reader.read_int32(0, true)? as usize;
    for _ in 0..count {
        if reader.skip_to_struct_begin(0, true)? {
            let ep = EndpointF::decode(reader)?;
            reader.skip_to_struct_end()?;
            result.push(ep);
        }
    }

    Ok(result)
}

/// QueryF interface methods
pub const QUERY_FIND_OBJECT_BY_ID: &str = "findObjectById";
pub const QUERY_FIND_OBJECT_BY_ID_4_ANY: &str = "findObjectById4Any";
pub const QUERY_FIND_OBJECT_BY_ID_4_ALL: &str = "findObjectById4All";
pub const QUERY_FIND_OBJECT_BY_ID_IN_SAME_GROUP: &str = "findObjectByIdInSameGroup";
pub const QUERY_FIND_OBJECT_BY_ID_IN_SAME_SET: &str = "findObjectByIdInSameSet";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_encode_decode() {
        let ep = EndpointF {
            host: "127.0.0.1".to_string(),
            port: 10000,
            timeout: 3000,
            istcp: 1,
            grid: 0,
            groupworkid: 0,
            grouprealid: 0,
            set_id: "".to_string(),
            qos: 0,
            bak_flag: 0,
            weight: 100,
            weight_type: 0,
            auth_type: 0,
        };

        let mut buf = Buffer::new();
        ep.encode(&mut buf).unwrap();

        let bytes = buf.to_bytes();
        let mut reader = Reader::new(&bytes);
        let decoded = EndpointF::decode(&mut reader).unwrap();

        assert_eq!(ep.host, decoded.host);
        assert_eq!(ep.port, decoded.port);
        assert_eq!(ep.timeout, decoded.timeout);
    }
}
