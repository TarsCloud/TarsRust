//! LogF protocol types for remote logging
//!
//! Corresponds to LogF.tars in TarsRust

use crate::codec::{Buffer, Reader};
use crate::Result;

/// LogInfo structure for log configuration
#[derive(Debug, Clone, Default)]
pub struct LogInfo {
    pub appname: String,           // Application name
    pub servername: String,        // Server name
    pub filename: String,          // Log filename
    pub format: String,            // Time format (e.g., "%Y%m%d")
    pub setdivision: String,       // SET division
    pub has_suffix: bool,          // Whether to add .log suffix
    pub has_app_prefix: bool,      // Whether to add app prefix
    pub has_square_bracket: bool,  // Whether to add [] around datetime
    pub concat_str: String,        // Concatenation string (default "_")
    pub separator: String,         // Log item separator (default "|")
    pub log_type: String,          // Log type (day/hour/minute)
}

impl LogInfo {
    pub fn new(app: &str, server: &str, filename: &str) -> Self {
        Self {
            appname: app.to_string(),
            servername: server.to_string(),
            filename: filename.to_string(),
            format: "%Y%m%d".to_string(),
            setdivision: String::new(),
            has_suffix: true,
            has_app_prefix: true,
            has_square_bracket: false,
            concat_str: "_".to_string(),
            separator: "|".to_string(),
            log_type: String::new(),
        }
    }

    pub fn encode(&self, buf: &mut Buffer) -> Result<()> {
        buf.write_string(&self.appname, 0)?;
        buf.write_string(&self.servername, 1)?;
        buf.write_string(&self.filename, 2)?;
        buf.write_string(&self.format, 3)?;
        buf.write_string(&self.setdivision, 4)?;
        buf.write_bool(self.has_suffix, 5)?;
        buf.write_bool(self.has_app_prefix, 6)?;
        buf.write_bool(self.has_square_bracket, 7)?;
        buf.write_string(&self.concat_str, 8)?;
        buf.write_string(&self.separator, 9)?;
        buf.write_string(&self.log_type, 10)?;
        Ok(())
    }

    pub fn decode(reader: &mut Reader) -> Result<Self> {
        let mut info = LogInfo::default();
        info.appname = reader.read_string(0, true)?;
        info.servername = reader.read_string(1, true)?;
        info.filename = reader.read_string(2, true)?;
        info.format = reader.read_string(3, true)?;
        info.setdivision = reader.read_string(4, false).unwrap_or_default();
        info.has_suffix = reader.read_bool(5, false).unwrap_or(true);
        info.has_app_prefix = reader.read_bool(6, false).unwrap_or(true);
        info.has_square_bracket = reader.read_bool(7, false).unwrap_or(false);
        info.concat_str = reader.read_string(8, false).unwrap_or_else(|_| "_".to_string());
        info.separator = reader.read_string(9, false).unwrap_or_else(|_| "|".to_string());
        info.log_type = reader.read_string(10, false).unwrap_or_default();
        Ok(info)
    }
}

/// Log interface methods
pub const LOG_LOGGER: &str = "logger";
pub const LOG_LOGGER_BY_INFO: &str = "loggerbyInfo";

/// Encode log buffer (vector<string>)
pub fn encode_log_buffer(buf: &mut Buffer, logs: &[String], tag: u8) -> Result<()> {
    buf.write_list(logs.len(), tag)?;
    for log in logs {
        buf.write_string(log, 0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_info_encode_decode() {
        let info = LogInfo::new("TestApp", "TestServer", "test_log");

        let mut buf = Buffer::new();
        info.encode(&mut buf).unwrap();

        let bytes = buf.to_bytes();
        let mut reader = Reader::new(&bytes);
        let decoded = LogInfo::decode(&mut reader).unwrap();

        assert_eq!(info.appname, decoded.appname);
        assert_eq!(info.servername, decoded.servername);
        assert_eq!(info.filename, decoded.filename);
    }
}
