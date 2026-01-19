//! StatF protocol types for statistics reporting
//!
//! Corresponds to StatF.tars in TarsRust

use crate::codec::{Buffer, Reader};
use crate::Result;
use std::collections::HashMap;

/// Time point distribution (milliseconds)
pub const TIME_POINTS: [i32; 9] = [5, 10, 50, 100, 200, 500, 1000, 2000, 3000];

/// Statistics message header - identifies a unique call chain
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub struct StatMicMsgHead {
    pub master_name: String,      // Caller module name
    pub slave_name: String,       // Callee module name
    pub interface_name: String,   // Interface name
    pub master_ip: String,        // Caller IP
    pub slave_ip: String,         // Callee IP
    pub slave_port: i32,          // Callee port
    pub return_value: i32,        // Return value
    pub slave_set_name: String,   // Callee SET name
    pub slave_set_area: String,   // Callee SET area
    pub slave_set_id: String,     // Callee SET group
    pub tars_version: String,     // Tars version
}

impl StatMicMsgHead {
    pub fn encode(&self, buf: &mut Buffer) -> Result<()> {
        buf.write_string(&self.master_name, 0)?;
        buf.write_string(&self.slave_name, 1)?;
        buf.write_string(&self.interface_name, 2)?;
        buf.write_string(&self.master_ip, 3)?;
        buf.write_string(&self.slave_ip, 4)?;
        buf.write_int32(self.slave_port, 5)?;
        buf.write_int32(self.return_value, 6)?;
        buf.write_string(&self.slave_set_name, 7)?;
        buf.write_string(&self.slave_set_area, 8)?;
        buf.write_string(&self.slave_set_id, 9)?;
        buf.write_string(&self.tars_version, 10)?;
        Ok(())
    }

    pub fn decode(reader: &mut Reader) -> Result<Self> {
        let mut head = StatMicMsgHead::default();
        head.master_name = reader.read_string(0, true)?;
        head.slave_name = reader.read_string(1, true)?;
        head.interface_name = reader.read_string(2, true)?;
        head.master_ip = reader.read_string(3, true)?;
        head.slave_ip = reader.read_string(4, true)?;
        head.slave_port = reader.read_int32(5, true)?;
        head.return_value = reader.read_int32(6, true)?;
        head.slave_set_name = reader.read_string(7, false).unwrap_or_default();
        head.slave_set_area = reader.read_string(8, false).unwrap_or_default();
        head.slave_set_id = reader.read_string(9, false).unwrap_or_default();
        head.tars_version = reader.read_string(10, false).unwrap_or_default();
        Ok(head)
    }
}

/// Statistics message body - aggregated metrics
#[derive(Debug, Clone, Default)]
pub struct StatMicMsgBody {
    pub count: i32,               // Success count
    pub timeout_count: i32,       // Timeout count
    pub exec_count: i32,          // Exception count
    pub interval_count: HashMap<i32, i32>,  // Time distribution
    pub total_rsp_time: i64,      // Total response time (ms)
    pub max_rsp_time: i32,        // Max response time
    pub min_rsp_time: i32,        // Min response time
}

impl StatMicMsgBody {
    pub fn new() -> Self {
        Self {
            count: 0,
            timeout_count: 0,
            exec_count: 0,
            interval_count: HashMap::new(),
            total_rsp_time: 0,
            max_rsp_time: 0,
            min_rsp_time: i32::MAX,
        }
    }

    /// Merge another body into this one
    pub fn merge(&mut self, other: &StatMicMsgBody) {
        self.count += other.count;
        self.timeout_count += other.timeout_count;
        self.exec_count += other.exec_count;
        self.total_rsp_time += other.total_rsp_time;
        if other.max_rsp_time > self.max_rsp_time {
            self.max_rsp_time = other.max_rsp_time;
        }
        if other.min_rsp_time < self.min_rsp_time {
            self.min_rsp_time = other.min_rsp_time;
        }
        for (k, v) in &other.interval_count {
            *self.interval_count.entry(*k).or_insert(0) += v;
        }
    }

    /// Add a response time to the distribution
    pub fn add_response_time(&mut self, rsp_time: i64) {
        let rsp_time_i32 = rsp_time as i32;

        // Update total
        self.total_rsp_time += rsp_time;

        // Update max/min
        if rsp_time_i32 > self.max_rsp_time {
            self.max_rsp_time = rsp_time_i32;
        }
        if rsp_time_i32 < self.min_rsp_time {
            self.min_rsp_time = rsp_time_i32;
        }

        // Update interval distribution
        for &point in &TIME_POINTS {
            if rsp_time_i32 < point {
                *self.interval_count.entry(point).or_insert(0) += 1;
                break;
            }
        }
    }

    pub fn encode(&self, buf: &mut Buffer) -> Result<()> {
        buf.write_int32(self.count, 0)?;
        buf.write_int32(self.timeout_count, 1)?;
        buf.write_int32(self.exec_count, 2)?;

        // Encode interval_count map<int32, int32>
        buf.write_map(self.interval_count.len(), 3)?;
        for (k, v) in &self.interval_count {
            buf.write_int32(*k, 0)?;
            buf.write_int32(*v, 1)?;
        }

        buf.write_int64(self.total_rsp_time, 4)?;
        buf.write_int32(self.max_rsp_time, 5)?;
        buf.write_int32(self.min_rsp_time, 6)?;
        Ok(())
    }

    pub fn decode(reader: &mut Reader) -> Result<Self> {
        let mut body = StatMicMsgBody::new();
        body.count = reader.read_int32(0, true)?;
        body.timeout_count = reader.read_int32(1, true)?;
        body.exec_count = reader.read_int32(2, true)?;
        // Skip interval_count decoding for now
        body.total_rsp_time = reader.read_int64(4, true)?;
        body.max_rsp_time = reader.read_int32(5, true)?;
        body.min_rsp_time = reader.read_int32(6, true)?;
        Ok(body)
    }
}

/// Full statistics message
#[derive(Debug, Clone)]
pub struct StatInfo {
    pub head: StatMicMsgHead,
    pub body: StatMicMsgBody,
}

impl StatInfo {
    pub fn new(head: StatMicMsgHead, body: StatMicMsgBody) -> Self {
        Self { head, body }
    }
}

/// StatF interface methods
pub const STAT_REPORT_MIC_MSG: &str = "reportMicMsg";
pub const STAT_REPORT_SAMPLE_MSG: &str = "reportSampleMsg";

/// Encode stat map for reporting
pub fn encode_stat_map(
    buf: &mut Buffer,
    stats: &HashMap<StatMicMsgHead, StatMicMsgBody>,
    tag: u8
) -> Result<()> {
    buf.write_map(stats.len(), tag)?;
    for (head, body) in stats {
        // Encode head as struct
        buf.write_struct_begin(0)?;
        head.encode(buf)?;
        buf.write_struct_end()?;

        // Encode body as struct
        buf.write_struct_begin(1)?;
        body.encode(buf)?;
        buf.write_struct_end()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_body_response_time() {
        let mut body = StatMicMsgBody::new();
        body.count = 1;
        body.add_response_time(35);  // Should go into 50ms bucket

        assert_eq!(body.total_rsp_time, 35);
        assert_eq!(body.max_rsp_time, 35);
        assert_eq!(body.min_rsp_time, 35);
        assert_eq!(body.interval_count.get(&50), Some(&1));
    }

    #[test]
    fn test_stat_body_merge() {
        let mut body1 = StatMicMsgBody::new();
        body1.count = 5;
        body1.timeout_count = 1;
        body1.total_rsp_time = 100;
        body1.max_rsp_time = 50;
        body1.min_rsp_time = 10;

        let mut body2 = StatMicMsgBody::new();
        body2.count = 3;
        body2.exec_count = 1;
        body2.total_rsp_time = 60;
        body2.max_rsp_time = 80;
        body2.min_rsp_time = 5;

        body1.merge(&body2);

        assert_eq!(body1.count, 8);
        assert_eq!(body1.timeout_count, 1);
        assert_eq!(body1.exec_count, 1);
        assert_eq!(body1.total_rsp_time, 160);
        assert_eq!(body1.max_rsp_time, 80);
        assert_eq!(body1.min_rsp_time, 5);
    }
}
