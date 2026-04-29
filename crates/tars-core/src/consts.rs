//! Constants used throughout the framework

/// Protocol versions
pub const TARS_VERSION: i16 = 1;
pub const TUP_VERSION: i16 = 2;
pub const JSON_VERSION: i16 = 3;

/// Packet types
pub const TARS_NORMAL: i8 = 0;
pub const TARS_ONEWAY: i8 = 1;

/// Message types
pub const TARS_MESSAGE_TYPE_NULL: i32 = 0;
pub const TARS_MESSAGE_TYPE_DYED: i32 = 4;
pub const TARS_MESSAGE_TYPE_TRACE: i32 = 8;

/// Return codes
pub const TARS_SERVER_SUCCESS: i32 = 0;
pub const TARS_SERVER_DECODE_ERR: i32 = -1;
pub const TARS_SERVER_QUEUE_TIMEOUT: i32 = -2;
pub const TARS_INVOKE_TIMEOUT: i32 = -3;
pub const TARS_SERVER_UNKNOWN_ERR: i32 = -99;

/// Transport protocols
pub const PROTO_TCP: i32 = 1;
pub const PROTO_UDP: i32 = 0;
pub const PROTO_SSL: i32 = 2;

/// Health check parameters
pub const FAIL_INTERVAL: u64 = 5;
pub const FAIL_N: i32 = 5;
pub const CHECK_TIME: u64 = 60;
pub const OVER_N: i32 = 2;
pub const FAIL_RATIO: f32 = 0.5;
pub const TRY_TIME_INTERVAL: u64 = 30;

/// Default timeouts (milliseconds)
pub const DEFAULT_ASYNC_TIMEOUT: u64 = 3000;
pub const DEFAULT_SYNC_TIMEOUT: u64 = 3000;
pub const DEFAULT_CONNECT_TIMEOUT: u64 = 3000;
pub const DEFAULT_IDLE_TIMEOUT: u64 = 600000;

/// Queue limits
pub const DEFAULT_QUEUE_LEN: usize = 10000;
pub const DEFAULT_MAX_INVOKE: i32 = 200000;

/// Max package length
pub const MAX_PACKAGE_LENGTH: u32 = 100 * 1024 * 1024;

/// Reconnect message
pub const RECONNECT_MSG: &str = "_reconnect_";

/// Status keys
pub const STATUS_DYED_KEY: &str = "STATUS_DYED_KEY";
pub const STATUS_TRACE_KEY: &str = "STATUS_TRACE_KEY";

/// Consistent hash virtual nodes
pub const CON_HASH_VIRTUAL_NODES: usize = 100;
