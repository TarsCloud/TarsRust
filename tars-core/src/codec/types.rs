//! Tars protocol type definitions

/// Tars data type constants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TarsType {
    /// int8 type
    Byte = 0,
    /// int16 type
    Short = 1,
    /// int32 type
    Int = 2,
    /// int64 type
    Long = 3,
    /// float32 type
    Float = 4,
    /// float64 type
    Double = 5,
    /// Short string (length < 256)
    String1 = 6,
    /// Long string (length >= 256)
    String4 = 7,
    /// Map type
    Map = 8,
    /// List/Vector type
    List = 9,
    /// Struct begin marker
    StructBegin = 10,
    /// Struct end marker
    StructEnd = 11,
    /// Zero value marker (optimization)
    ZeroTag = 12,
    /// Simple list ([]byte)
    SimpleList = 13,
}

impl TarsType {
    /// Create TarsType from u8 value
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TarsType::Byte),
            1 => Some(TarsType::Short),
            2 => Some(TarsType::Int),
            3 => Some(TarsType::Long),
            4 => Some(TarsType::Float),
            5 => Some(TarsType::Double),
            6 => Some(TarsType::String1),
            7 => Some(TarsType::String4),
            8 => Some(TarsType::Map),
            9 => Some(TarsType::List),
            10 => Some(TarsType::StructBegin),
            11 => Some(TarsType::StructEnd),
            12 => Some(TarsType::ZeroTag),
            13 => Some(TarsType::SimpleList),
            _ => None,
        }
    }

    /// Get the u8 value of this type
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl From<TarsType> for u8 {
    fn from(ty: TarsType) -> Self {
        ty as u8
    }
}

impl TryFrom<u8> for TarsType {
    type Error = crate::TarsError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        TarsType::from_u8(value).ok_or_else(|| {
            crate::TarsError::Codec(format!("Invalid Tars type: {}", value))
        })
    }
}

/// Head information containing type and tag
#[derive(Debug, Clone, Copy)]
pub struct Head {
    /// Data type
    pub ty: TarsType,
    /// Field tag/identifier
    pub tag: u8,
}

impl Head {
    /// Create a new Head
    pub fn new(ty: TarsType, tag: u8) -> Self {
        Self { ty, tag }
    }

    /// Check if this is a zero value marker
    pub fn is_zero(&self) -> bool {
        self.ty == TarsType::ZeroTag
    }

    /// Check if this is a struct end marker
    pub fn is_struct_end(&self) -> bool {
        self.ty == TarsType::StructEnd
    }
}

/// Package parse result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStatus {
    /// Package is complete
    Full,
    /// Package data is incomplete (need more bytes)
    Less,
    /// Package is invalid
    Error,
}

/// Parse Tars request package header
/// Returns (package_length, status)
pub fn parse_package(data: &[u8]) -> (usize, PackageStatus) {
    if data.len() < 4 {
        return (0, PackageStatus::Less);
    }

    let header_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

    if header_len < 4 || header_len > crate::consts::MAX_PACKAGE_LENGTH as usize {
        return (0, PackageStatus::Error);
    }

    if data.len() < header_len {
        return (0, PackageStatus::Less);
    }

    (header_len, PackageStatus::Full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tars_type_conversion() {
        assert_eq!(TarsType::Byte.as_u8(), 0);
        assert_eq!(TarsType::from_u8(0), Some(TarsType::Byte));
        assert_eq!(TarsType::from_u8(99), None);
    }

    #[test]
    fn test_parse_package() {
        // Too short
        let (_, status) = parse_package(&[0, 0, 1]);
        assert_eq!(status, PackageStatus::Less);

        // Complete package (length = 8)
        let data = [0, 0, 0, 8, 1, 2, 3, 4];
        let (len, status) = parse_package(&data);
        assert_eq!(status, PackageStatus::Full);
        assert_eq!(len, 8);

        // Incomplete
        let data = [0, 0, 0, 10, 1, 2, 3, 4];
        let (_, status) = parse_package(&data);
        assert_eq!(status, PackageStatus::Less);
    }
}
