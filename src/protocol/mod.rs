//! # Protocol Module
//!
//! This module defines the Tars protocol structures including RequestPacket and ResponsePacket.
//!
//! ## Protocol Overview
//!
//! The Tars protocol uses a binary format with a 4-byte length prefix followed by the packet body.
//!
//! ### RequestPacket Fields
//!
//! | Tag | Field | Type | Description |
//! |-----|-------|------|-------------|
//! | 1 | iVersion | int16 | Protocol version |
//! | 2 | cPacketType | int8 | Packet type (0=normal, 1=oneway) |
//! | 3 | iMessageType | int32 | Message type flags |
//! | 4 | iRequestId | int32 | Request ID |
//! | 5 | sServantName | string | Service name |
//! | 6 | sFuncName | string | Method name |
//! | 7 | sBuffer | bytes | Request data |
//! | 8 | iTimeout | int32 | Timeout in ms |
//! | 9 | context | map<string,string> | Context |
//! | 10 | status | map<string,string> | Status |

mod packet;
mod consts;
pub mod queryf;
pub mod logf;
pub mod statf;

pub use packet::{RequestPacket, ResponsePacket};
pub use consts::*;
pub use queryf::EndpointF;
pub use logf::LogInfo;
pub use statf::{StatMicMsgHead, StatMicMsgBody, StatInfo};

use crate::{Result, codec};

/// Protocol interface for client-side encoding/decoding
pub trait Protocol: Send + Sync {
    /// Parse package boundary, returns (length, status)
    fn parse_package(&self, buff: &[u8]) -> (usize, codec::PackageStatus);

    /// Encode request packet
    fn request_pack(&self, req: &RequestPacket) -> Result<Vec<u8>>;

    /// Decode response packet
    fn response_unpack(&self, pkg: &[u8]) -> Result<ResponsePacket>;
}

/// Default Tars protocol implementation
#[derive(Debug, Default, Clone)]
pub struct TarsProtocol;

impl TarsProtocol {
    pub fn new() -> Self {
        Self
    }
}

impl Protocol for TarsProtocol {
    fn parse_package(&self, buff: &[u8]) -> (usize, codec::PackageStatus) {
        codec::parse_package(buff)
    }

    fn request_pack(&self, req: &RequestPacket) -> Result<Vec<u8>> {
        req.encode()
    }

    fn response_unpack(&self, pkg: &[u8]) -> Result<ResponsePacket> {
        ResponsePacket::decode(pkg)
    }
}

/// Server protocol interface
pub trait ServerProtocol: Send + Sync {
    /// Parse package boundary
    fn parse_package(&self, buff: &[u8]) -> (usize, codec::PackageStatus);

    /// Handle request and return response
    fn invoke(&self, ctx: &mut crate::util::Context, pkg: &[u8]) -> Vec<u8>;

    /// Handle timeout
    fn invoke_timeout(&self, pkg: &[u8]) -> Vec<u8>;

    /// Get close message
    fn get_close_msg(&self) -> Vec<u8>;

    /// Handle connection close
    fn do_close(&self, ctx: &crate::util::Context);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_packet_encode_decode() {
        let mut req = RequestPacket::new();
        req.i_version = crate::consts::TARS_VERSION;
        req.c_packet_type = crate::consts::TARS_NORMAL;
        req.i_request_id = 12345;
        req.s_servant_name = "Test.HelloServer.HelloObj".to_string();
        req.s_func_name = "sayHello".to_string();
        req.s_buffer = vec![1, 2, 3, 4];
        req.i_timeout = 3000;
        req.context.insert("key".to_string(), "value".to_string());

        let encoded = req.encode().unwrap();

        // Decode
        let decoded = RequestPacket::decode(&encoded).unwrap();

        assert_eq!(decoded.i_version, req.i_version);
        assert_eq!(decoded.c_packet_type, req.c_packet_type);
        assert_eq!(decoded.i_request_id, req.i_request_id);
        assert_eq!(decoded.s_servant_name, req.s_servant_name);
        assert_eq!(decoded.s_func_name, req.s_func_name);
        assert_eq!(decoded.s_buffer, req.s_buffer);
        assert_eq!(decoded.i_timeout, req.i_timeout);
        assert_eq!(decoded.context.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_packet_encode_decode() {
        let mut rsp = ResponsePacket::new();
        rsp.i_version = crate::consts::TARS_VERSION;
        rsp.i_request_id = 12345;
        rsp.i_ret = 0;
        rsp.s_buffer = vec![5, 6, 7, 8];
        rsp.s_result_desc = "success".to_string();

        let encoded = rsp.encode().unwrap();

        // Decode
        let decoded = ResponsePacket::decode(&encoded).unwrap();

        assert_eq!(decoded.i_version, rsp.i_version);
        assert_eq!(decoded.i_request_id, rsp.i_request_id);
        assert_eq!(decoded.i_ret, rsp.i_ret);
        assert_eq!(decoded.s_buffer, rsp.s_buffer);
        assert_eq!(decoded.s_result_desc, rsp.s_result_desc);
    }
}
