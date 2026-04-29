//! Protocol constants

/// Tars protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum TarsVersion {
    /// Standard Tars protocol
    Tars = 1,
    /// TUP protocol
    Tup = 2,
    /// JSON protocol
    Json = 3,
}

impl TarsVersion {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(TarsVersion::Tars),
            2 => Some(TarsVersion::Tup),
            3 => Some(TarsVersion::Json),
            _ => None,
        }
    }

    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

impl From<TarsVersion> for i16 {
    fn from(v: TarsVersion) -> Self {
        v as i16
    }
}

/// Packet type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum PacketType {
    /// Normal request/response
    Normal = 0,
    /// Oneway request (no response)
    Oneway = 1,
}

impl PacketType {
    pub fn from_i8(value: i8) -> Option<Self> {
        match value {
            0 => Some(PacketType::Normal),
            1 => Some(PacketType::Oneway),
            _ => None,
        }
    }

    pub fn as_i8(self) -> i8 {
        self as i8
    }
}

impl From<PacketType> for i8 {
    fn from(p: PacketType) -> Self {
        p as i8
    }
}

/// Message type flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageType(pub i32);

impl MessageType {
    pub const NULL: MessageType = MessageType(0);
    pub const DYED: MessageType = MessageType(4);
    pub const TRACE: MessageType = MessageType(8);

    pub fn has_flag(&self, flag: MessageType) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn add_flag(&mut self, flag: MessageType) {
        self.0 |= flag.0;
    }

    pub fn remove_flag(&mut self, flag: MessageType) {
        self.0 &= !flag.0;
    }
}

/// Return codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ReturnCode {
    /// Success
    Success = 0,
    /// Decode error
    DecodeError = -1,
    /// Queue timeout
    QueueTimeout = -2,
    /// Invoke timeout
    InvokeTimeout = -3,
    /// Unknown error
    UnknownError = -99,
}

impl ReturnCode {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(ReturnCode::Success),
            -1 => Some(ReturnCode::DecodeError),
            -2 => Some(ReturnCode::QueueTimeout),
            -3 => Some(ReturnCode::InvokeTimeout),
            -99 => Some(ReturnCode::UnknownError),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }

    pub fn is_success(value: i32) -> bool {
        value == 0
    }
}

impl From<ReturnCode> for i32 {
    fn from(r: ReturnCode) -> Self {
        r as i32
    }
}

/// Transport protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TransportProtocol {
    /// UDP protocol
    Udp = 0,
    /// TCP protocol
    Tcp = 1,
    /// SSL/TLS protocol
    Ssl = 2,
}

impl TransportProtocol {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(TransportProtocol::Udp),
            1 => Some(TransportProtocol::Tcp),
            2 => Some(TransportProtocol::Ssl),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TransportProtocol::Udp => "udp",
            TransportProtocol::Tcp => "tcp",
            TransportProtocol::Ssl => "ssl",
        }
    }
}

impl From<TransportProtocol> for i32 {
    fn from(p: TransportProtocol) -> Self {
        p as i32
    }
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tars_version() {
        assert_eq!(TarsVersion::Tars.as_i16(), 1);
        assert_eq!(TarsVersion::from_i16(1), Some(TarsVersion::Tars));
        assert_eq!(TarsVersion::from_i16(99), None);
    }

    #[test]
    fn test_packet_type() {
        assert_eq!(PacketType::Normal.as_i8(), 0);
        assert_eq!(PacketType::Oneway.as_i8(), 1);
        assert_eq!(PacketType::from_i8(0), Some(PacketType::Normal));
    }

    #[test]
    fn test_message_type() {
        let mut mt = MessageType::NULL;
        assert!(!mt.has_flag(MessageType::DYED));

        mt.add_flag(MessageType::DYED);
        assert!(mt.has_flag(MessageType::DYED));
        assert!(!mt.has_flag(MessageType::TRACE));

        mt.add_flag(MessageType::TRACE);
        assert!(mt.has_flag(MessageType::DYED));
        assert!(mt.has_flag(MessageType::TRACE));

        mt.remove_flag(MessageType::DYED);
        assert!(!mt.has_flag(MessageType::DYED));
        assert!(mt.has_flag(MessageType::TRACE));
    }

    #[test]
    fn test_return_code() {
        assert!(ReturnCode::is_success(0));
        assert!(!ReturnCode::is_success(-1));
        assert_eq!(ReturnCode::Success.as_i32(), 0);
    }

    #[test]
    fn test_transport_protocol() {
        assert_eq!(TransportProtocol::Tcp.as_str(), "tcp");
        assert_eq!(TransportProtocol::from_i32(1), Some(TransportProtocol::Tcp));
    }
}
