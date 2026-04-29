# Cargo Workspace 拆分实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `tars-rs` 从单 crate 重构为 7 成员 Cargo workspace,通过 facade `tars` crate 保持公开 API 不变。

**Architecture:** 自底向上 —— 先在原单 crate 内消除 `endpoint` ↔ `util` 的循环依赖(把解析函数搬进 `endpoint`),然后建立 workspace 骨架,再按层逐个把模块迁入目标 crate(`tars-core` → `tars-protocol` → `tars-util` → `tars-transport` → `tars-registry` → `tars-rpc` → `tars` facade)。每个 crate 通过 `cargo build -p <crate>` 独立验证。

**Tech Stack:** Rust 2021,Cargo workspaces。

**基线确认:** 当前 `cargo build` 通过,`cargo test` 110 passed / 3 ignored。

---

## 文件结构(终态)

```
tars-rs/
├── Cargo.toml                                  # virtual workspace
├── crates/
│   ├── tars-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── codec/                          # ← src/codec/
│   │       │   ├── buffer.rs
│   │       │   ├── mod.rs
│   │       │   ├── reader.rs
│   │       │   └── types.rs
│   │       ├── consts.rs                       # ← extracted from src/lib.rs
│   │       ├── error.rs                        # ← extracted from src/lib.rs
│   │       └── lib.rs
│   ├── tars-protocol/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── protocol/                       # ← src/protocol/
│   │       ├── endpoint/                       # ← src/endpoint/(已含搬进的 parse_*)
│   │       └── lib.rs
│   ├── tars-util/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── util/                           # ← src/util/(已剥离 parse_*)
│   │       ├── selector/                       # ← src/selector/
│   │       └── lib.rs
│   ├── tars-transport/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── transport/                      # ← src/transport/
│   │       ├── filter/                         # ← src/filter/
│   │       └── lib.rs
│   ├── tars-registry/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── registry/                       # ← src/registry/
│   │       ├── adapter/                        # ← src/adapter/
│   │       ├── logger/                         # ← src/logger/
│   │       ├── stat/                           # ← src/stat/
│   │       └── lib.rs
│   ├── tars-rpc/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── servant/                        # ← src/servant/
│   │       ├── communicator/                   # ← src/communicator/
│   │       ├── application/                    # ← src/application/
│   │       └── lib.rs
│   └── tars/
│       ├── Cargo.toml
│       ├── examples/
│       │   └── client.rs                       # ← examples/client.rs
│       └── src/
│           └── lib.rs                          # facade
└── examples/
    └── hello/                                  # 保留(Go demo)
```

---

## Task 1: Pre-flight — 消除 `endpoint` ↔ `util` 循环依赖

**问题:** 当前 `src/endpoint/mod.rs:145` 调用 `crate::util::parse_endpoint_string`,而 `src/util/mod.rs::parse_endpoint_string` 又依赖 `crate::endpoint::Endpoint`、`crate::protocol::TransportProtocol`。同一 crate 里没问题,但拆分后 `tars-protocol`(含 endpoint)与 `tars-util` 会互相依赖。

**修复方案:** 把 `parse_endpoint_string` 与 `parse_obj_name` 从 `src/util/mod.rs` 搬进 `src/endpoint/mod.rs`(连同对应的 2 个测试)。这样修复后,`util` 不再引用 `Endpoint`/`TransportProtocol`,循环消除。该函数本质上就是构造 `Endpoint`,放在 endpoint 模块语义更顺。

**Files:**
- Modify: `src/util/mod.rs`(删除 `parse_endpoint_string`、`parse_obj_name`,删除对应测试 `test_parse_endpoint_string`、`test_parse_obj_name`)
- Modify: `src/endpoint/mod.rs`(新增上述两函数和它们的测试,删除现有 `from_string` 中的 `crate::util::` 调用)
- Modify: `src/communicator/mod.rs:13` 和 `src/lib.rs`(若有)中的 `use crate::util::parse_obj_name` 改为 `use crate::endpoint::parse_obj_name`

- [ ] **Step 1: 在 `src/endpoint/mod.rs` 末尾(`#[cfg(test)] mod tests` 之前)新增以下函数**

```rust
/// Parse endpoint string like "tcp -h 127.0.0.1 -p 10000 -t 3000"
pub fn parse_endpoint_string(s: &str) -> Option<Endpoint> {
    let parts: Vec<&str> = s.trim().split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let proto = match parts[0].to_lowercase().as_str() {
        "tcp" => TransportProtocol::Tcp,
        "udp" => TransportProtocol::Udp,
        "ssl" => TransportProtocol::Ssl,
        _ => return None,
    };

    let mut host = String::new();
    let mut port: u16 = 0;
    let mut timeout: u64 = 3000;

    let mut i = 1;
    while i < parts.len() {
        match parts[i] {
            "-h" if i + 1 < parts.len() => {
                host = parts[i + 1].to_string();
                i += 2;
            }
            "-p" if i + 1 < parts.len() => {
                port = parts[i + 1].parse().unwrap_or(0);
                i += 2;
            }
            "-t" if i + 1 < parts.len() => {
                timeout = parts[i + 1].parse().unwrap_or(3000);
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    if host.is_empty() || port == 0 {
        return None;
    }

    Some(Endpoint {
        host,
        port,
        timeout,
        istcp: proto.as_i32(),
        ..Default::default()
    })
}

/// Parse object name with endpoints
/// Format: "App.Server.Obj" or "App.Server.Obj@tcp -h 127.0.0.1 -p 10000"
pub fn parse_obj_name(obj_name: &str) -> (String, Vec<Endpoint>) {
    let parts: Vec<&str> = obj_name.splitn(2, '@').collect();
    let name = parts[0].to_string();

    let endpoints = if parts.len() > 1 {
        parts[1]
            .split(':')
            .filter_map(|s| parse_endpoint_string(s.trim()))
            .collect()
    } else {
        Vec::new()
    };

    (name, endpoints)
}
```

- [ ] **Step 2: 把 `src/endpoint/mod.rs:144-146` 的 `Endpoint::from_string` 改为本地调用**

把:
```rust
pub fn from_string(s: &str) -> Option<Self> {
    crate::util::parse_endpoint_string(s)
}
```
改为:
```rust
pub fn from_string(s: &str) -> Option<Self> {
    parse_endpoint_string(s)
}
```

- [ ] **Step 3: 把测试一起搬过来。在 `src/endpoint/mod.rs` 的 `mod tests` 内追加以下两个测试**

```rust
    #[test]
    fn test_parse_endpoint_string() {
        let ep = parse_endpoint_string("tcp -h 127.0.0.1 -p 10000 -t 5000").unwrap();
        assert_eq!(ep.host, "127.0.0.1");
        assert_eq!(ep.port, 10000);
        assert_eq!(ep.timeout, 5000);
        assert_eq!(ep.istcp, 1);

        let ep = parse_endpoint_string("udp -h 192.168.1.1 -p 8080").unwrap();
        assert_eq!(ep.host, "192.168.1.1");
        assert_eq!(ep.port, 8080);
        assert_eq!(ep.istcp, 0);
    }

    #[test]
    fn test_parse_obj_name() {
        let (name, eps) = parse_obj_name("Test.HelloServer.HelloObj");
        assert_eq!(name, "Test.HelloServer.HelloObj");
        assert!(eps.is_empty());

        let (name, eps) = parse_obj_name("Test.HelloServer.HelloObj@tcp -h 127.0.0.1 -p 10000");
        assert_eq!(name, "Test.HelloServer.HelloObj");
        assert_eq!(eps.len(), 1);
        assert_eq!(eps[0].host, "127.0.0.1");
        assert_eq!(eps[0].port, 10000);

        let (name, eps) = parse_obj_name("Test.HelloServer.HelloObj@tcp -h 127.0.0.1 -p 10000:tcp -h 127.0.0.1 -p 10001");
        assert_eq!(name, "Test.HelloServer.HelloObj");
        assert_eq!(eps.len(), 2);
    }
```

- [ ] **Step 4: 从 `src/util/mod.rs` 删除 `parse_endpoint_string`(行 40-93)和 `parse_obj_name`(行 95-111)、对应测试 `test_parse_endpoint_string`(行 134-146)和 `test_parse_obj_name`(行 148-163)**

删除后,`src/util/mod.rs` 中不再 `use crate::endpoint`、`use crate::protocol`。剩余内容只保留 `Context`、`config::*`、`gen_request_id`、`bytes_to_int8_slice`、`int8_slice_to_bytes` 及它们的测试。

- [ ] **Step 5: 修改 `src/communicator/mod.rs:13`**

把:
```rust
use crate::util::{ClientConfig, parse_obj_name};
```
改为:
```rust
use crate::util::ClientConfig;
use crate::endpoint::parse_obj_name;
```

- [ ] **Step 6: 验证全量构建与测试**

```bash
cargo build
cargo test
```
Expected:`cargo build` 成功,`cargo test` 测试数仍为 110 passed / 3 ignored。

- [ ] **Step 7: Commit**

```bash
git add src/endpoint/mod.rs src/util/mod.rs src/communicator/mod.rs
git commit -m "refactor: move endpoint parsing helpers from util to endpoint

Eliminates the endpoint↔util cyclic dependency in preparation for the
upcoming workspace split where these modules will live in different crates."
```

---

## Task 2: 替换根 `Cargo.toml` 为 virtual workspace

**Files:**
- Modify: `Cargo.toml`(完整重写)

- [ ] **Step 1: 把 `Cargo.toml` 整文件覆盖为以下内容**

```toml
[workspace]
resolver = "2"
members = [
    "crates/tars-core",
    "crates/tars-protocol",
    "crates/tars-util",
    "crates/tars-transport",
    "crates/tars-registry",
    "crates/tars-rpc",
    "crates/tars",
]
default-members = ["crates/tars"]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["TarsRust Team"]
license = "BSD-3-Clause"
repository = "https://github.com/TarsCloud/TarsRust"

[workspace.dependencies]
# Internal crates
tars-core = { path = "crates/tars-core", version = "0.1.0" }
tars-protocol = { path = "crates/tars-protocol", version = "0.1.0" }
tars-util = { path = "crates/tars-util", version = "0.1.0" }
tars-transport = { path = "crates/tars-transport", version = "0.1.0" }
tars-registry = { path = "crates/tars-registry", version = "0.1.0" }
tars-rpc = { path = "crates/tars-rpc", version = "0.1.0" }

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Networking
tokio-rustls = "0.25"
rustls = "0.22"
rustls-pemfile = "2.0"
webpki-roots = "0.26"

# Serialization
bytes = "1.5"
byteorder = "1.5"

# Errors / utilities
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
parking_lot = "0.12"
dashmap = "5.5"
once_cell = "1.19"
rand = "0.8"
crc32fast = "1.3"
md-5 = "0.10"
hex = "0.4"

# Time
chrono = "0.4"

# Async traits
async-trait = "0.1"

# Config / serde
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Signals
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }

# Atomics
crossbeam = "0.8"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

注意:不再有顶层 `[package]`、`[dependencies]`、`[dev-dependencies]`、`[features]`、`[[example]]`。空 feature `tls`、`opentelemetry` 与未使用的 `tokio-test` 一并删除。

- [ ] **Step 2: 验证 workspace 解析**

```bash
cargo metadata --no-deps --format-version 1 > /dev/null
```
Expected:命令成功(0 退出码),`Cargo.lock` 可能略有变化。
此时项目尚不可构建(成员 crate 的目录还未创建),这是预期。

- [ ] **Step 3: 暂不 commit。** 在 Task 9 之前,源码处于过渡状态;到 Task 9 末整体构建通过后再做一次大的 commit。

---

## Task 3: 创建 `tars-core`(error / consts / codec / Context)

**说明:** `Context` 原在 `src/util/context.rs`,但被 `protocol::ServerProtocol` trait 与 `transport`、`filter`、`servant` 多层引用。如果把它留在 `tars-util`,`tars-protocol` 就需要反过来依赖 `tars-util`,形成循环。把它上提到 `tars-core` 既消除循环,也避免业务上层强行依赖 util。这与 spec 风险章节"循环里最小公共类型上提到底层 crate"一致。

**Files:**
- Create: `crates/tars-core/Cargo.toml`
- Create: `crates/tars-core/src/lib.rs`
- Create: `crates/tars-core/src/error.rs`
- Create: `crates/tars-core/src/consts.rs`
- Move: `src/codec/` → `crates/tars-core/src/codec/`
- Move: `src/util/context.rs` → `crates/tars-core/src/context.rs`
- Modify: `src/util/mod.rs`(去掉 `mod context;` / `pub use context::Context;`)

- [ ] **Step 1: 创建目录骨架**

```bash
mkdir -p crates/tars-core/src
```

- [ ] **Step 2: 写 `crates/tars-core/Cargo.toml`**

```toml
[package]
name = "tars-core"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Core types (error, consts) and Tars binary codec"

[dependencies]
bytes.workspace = true
byteorder.workspace = true
thiserror.workspace = true
parking_lot.workspace = true
```

(`parking_lot` 是因为 `context.rs` 用了 `RwLock`。)

- [ ] **Step 3: 写 `crates/tars-core/src/error.rs`**

把当前 `src/lib.rs` 中 `pub mod error { ... }` 内层(`use thiserror::Error;` 起,到 `pub type Result<T> = ...` 止)整体复制为该文件。

```rust
//! Error types for the Tars framework

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TarsError {
    #[error("Codec error: {0}")]
    Codec(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Transport error: {0}")]
    Transport(#[from] std::io::Error),

    #[error("Timeout error: operation timed out after {0}ms")]
    Timeout(u64),

    #[error("No available endpoint")]
    NoEndpoint,

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Server error: code={code}, message={message}")]
    ServerError { code: i32, message: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Queue full")]
    QueueFull,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, TarsError>;
```

- [ ] **Step 4: 写 `crates/tars-core/src/consts.rs`**

把当前 `src/lib.rs` 中 `pub mod consts { ... }` 的整个内层(从 `/// Protocol versions` 起到 `CON_HASH_VIRTUAL_NODES` 止)复制过来,文件首行写 `//! Constants used throughout the framework`。

```rust
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
```

- [ ] **Step 5: 移动 `codec` 目录与 `Context`**

```bash
git mv src/codec crates/tars-core/src/codec
git mv src/util/context.rs crates/tars-core/src/context.rs
```

`crates/tars-core/src/context.rs` 内部不引用任何 `crate::xxx`(已 grep 确认),无需改动。

- [ ] **Step 6: 同步修改 `src/util/mod.rs`**

把文件最上方的:
```rust
mod context;
mod config;

pub use context::Context;
pub use config::*;
```
改为:
```rust
mod config;

pub use config::*;
```

- [ ] **Step 7: 写 `crates/tars-core/src/lib.rs`**

```rust
//! # tars-core
//!
//! Core types (error / Result / consts), request Context, and the Tars binary codec.

pub mod codec;
pub mod consts;
pub mod context;
pub mod error;

pub use context::Context;
pub use error::{Result, TarsError};
```

- [ ] **Step 8: 验证 `tars-core` 可独立编译**

```bash
cargo build -p tars-core
```
Expected:成功。`codec/*` 中的 `crate::Result`、`crate::TarsError`、`crate::consts::*` 都仍可解析,因为它们现在指向 `tars-core` 自己。

- [ ] **Step 9: 运行 `tars-core` 的测试**

```bash
cargo test -p tars-core
```
Expected:所有 codec 单元测试通过(reader/buffer 的 tests 都在 `crate::codec::Buffer` 路径上,在新 crate 内仍可达)。

---

## Task 4: 创建 `tars-protocol`(protocol / endpoint)

**Files:**
- Create: `crates/tars-protocol/Cargo.toml`
- Create: `crates/tars-protocol/src/lib.rs`
- Move: `src/protocol/` → `crates/tars-protocol/src/protocol/`
- Move: `src/endpoint/` → `crates/tars-protocol/src/endpoint/`
- Modify: 上述被移文件中所有 `use crate::xxx`(明细见步骤)

- [ ] **Step 1: 创建目录与 manifest**

```bash
mkdir -p crates/tars-protocol/src
```

`crates/tars-protocol/Cargo.toml`:
```toml
[package]
name = "tars-protocol"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Tars wire protocol packets and endpoint types"

[dependencies]
tars-core.workspace = true
bytes.workspace = true
byteorder.workspace = true
async-trait.workspace = true
```

(`async-trait` 是因为 `protocol/mod.rs` 里有 `#[async_trait]` trait。)

- [ ] **Step 2: 移动 `protocol` 与 `endpoint`**

```bash
git mv src/protocol crates/tars-protocol/src/protocol
git mv src/endpoint crates/tars-protocol/src/endpoint
```

- [ ] **Step 3: 写 `crates/tars-protocol/src/lib.rs`**

```rust
//! # tars-protocol
//!
//! Tars wire protocol packets and endpoint definitions.

pub mod endpoint;
pub mod protocol;

pub use endpoint::{Endpoint, ServantInstance, WeightType, parse_endpoint_string, parse_obj_name};
pub use protocol::{
    EndpointF, LogInfo, RequestPacket, ResponsePacket, StatInfo, StatMicMsgBody, StatMicMsgHead,
};
// PacketType / TarsVersion / TransportProtocol live in protocol/consts.rs and
// reach the crate root via `pub use consts::*` in protocol/mod.rs.
pub use protocol::{PacketType, TarsVersion, TransportProtocol};
```

(列表与 `src/lib.rs:48-49` 的 `pub use protocol::{...}` 完全对应。`PacketType`/`TarsVersion` 来自 `protocol/consts.rs` 的 `pub use consts::*`;`TransportProtocol` 也是同样途径,被 `endpoint::Endpoint` 内部引用。)

- [ ] **Step 4: 改写 `protocol/packet.rs`**

将首部:
```rust
use crate::{Result, codec::{Buffer, Reader}};
```
改为:
```rust
use tars_core::{Result, codec::{Buffer, Reader}};
```

把文件内 `crate::consts::XXX` 全部替换为 `tars_core::consts::XXX`(出现处:行 35/36/42/60/146/147/161/162/165/176/177/192/199/265/272/274/275/276/278/279/280/303)。可使用如下命令批量替换(或在编辑器中用 replace_all):

```bash
sed -i '' 's/crate::consts::/tars_core::consts::/g' crates/tars-protocol/src/protocol/packet.rs
```

- [ ] **Step 5: 改写 `protocol/mod.rs`**

行 36:
```rust
use crate::{Result, codec};
```
改为:
```rust
use tars_core::{Result, codec};
```

行 80, 89(trait 方法签名)中的 `crate::util::Context` 改为 `tars_core::Context`(在 Task 3 已把 Context 移到 tars-core):

```rust
fn invoke(&self, ctx: &mut tars_core::Context, pkg: &[u8]) -> Vec<u8>;
...
fn do_close(&self, ctx: &tars_core::Context);
```

行 99, 100, 126 测试中的 `crate::consts::XXX` → `tars_core::consts::XXX`。批量替换:
```bash
sed -i '' 's/crate::consts::/tars_core::consts::/g' crates/tars-protocol/src/protocol/mod.rs
```

- [ ] **Step 6: 改写 `protocol/logf.rs` / `queryf.rs` / `statf.rs`**

3 个文件均把:
```rust
use crate::codec::{Buffer, Reader};
use crate::Result;
```
改为:
```rust
use tars_core::codec::{Buffer, Reader};
use tars_core::Result;
```

`protocol/consts.rs` 无 `use crate::`,无需改动。

- [ ] **Step 7: 改写 `endpoint/mod.rs` 的 `use crate::` 引用**

`crates/tars-protocol/src/endpoint/mod.rs`:行 7:
```rust
use crate::protocol::TransportProtocol;
```
保持不变(`TransportProtocol` 在同一 crate 的 `protocol` 模块,`crate::protocol::` 仍指 `tars-protocol::protocol`)。

(Task 1 已经把 `Endpoint::from_string` 改成本地 `parse_endpoint_string` 调用,无需再改。)

- [ ] **Step 8: 验证**

```bash
cargo build -p tars-protocol
cargo test -p tars-protocol
```
Expected:构建通过,所有原 protocol/endpoint 单元测试通过(包括 Task 1 加入 endpoint 的 `test_parse_endpoint_string`、`test_parse_obj_name`)。

---

## Task 5: 创建 `tars-util`(util / selector)

**Files:**
- Create: `crates/tars-util/Cargo.toml`
- Create: `crates/tars-util/src/lib.rs`
- Move: `src/util/`(剩余的 mod.rs、config.rs)→ `crates/tars-util/src/util/`
- Move: `src/selector/` → `crates/tars-util/src/selector/`

- [ ] **Step 1: 创建目录与 manifest**

```bash
mkdir -p crates/tars-util/src
```

`crates/tars-util/Cargo.toml`:
```toml
[package]
name = "tars-util"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Shared utilities (configs, request id) and load-balancing selectors"

[dependencies]
tars-core.workspace = true
tars-protocol.workspace = true
parking_lot.workspace = true
once_cell.workspace = true
rand.workspace = true
crc32fast.workspace = true
serde.workspace = true
toml.workspace = true
async-trait.workspace = true
```

(具体三方依赖按各 mod.rs 头部 `use` 列表调整;若 cargo build 报缺失,补齐即可。)

- [ ] **Step 2: 移动 `util` 与 `selector`**

```bash
git mv src/util crates/tars-util/src/util
git mv src/selector crates/tars-util/src/selector
```

(`util/context.rs` 已在 Task 4 提前移到 `tars-core`,此处只移剩余 `mod.rs`、`config.rs`。)

- [ ] **Step 3: 写 `crates/tars-util/src/lib.rs`**

```rust
//! # tars-util
//!
//! Configs, request-id generator, byte helpers and load-balancing selectors.

pub mod selector;
pub mod util;

pub use selector::{HashType, Selector, create_selector};
pub use util::{ClientConfig, ServerConfig, gen_request_id, bytes_to_int8_slice, int8_slice_to_bytes};
```

(若 `util/config.rs` 暴露的名字超出 ClientConfig/ServerConfig,补齐到 `pub use util::*`。)

- [ ] **Step 4: 修复 `util/mod.rs` 的引用**

`crates/tars-util/src/util/mod.rs`:
- 删除文件最上方的 `mod context;` / `pub use context::Context;`(`context` 已搬到 tars-core)
- 该文件无任何 `use crate::endpoint`/`use crate::protocol`(Task 1 已剥离)
- 保留 `mod config; pub use config::*;`

- [ ] **Step 5: 修复 `selector/*.rs` 的引用**

逐文件批量替换:

| 文件 | 原 | 新 |
|---|---|---|
| `selector/mod.rs:24` | `use crate::{Endpoint, Result};` | `use tars_protocol::Endpoint;\nuse tars_core::Result;` |
| `selector/random.rs:5` | `use crate::{Endpoint, Result, TarsError};` | `use tars_protocol::Endpoint;\nuse tars_core::{Result, TarsError};` |
| `selector/random.rs:77` (test) | `use crate::selector::DefaultMessage;` | `use crate::selector::DefaultMessage;`(同 crate 内,保持) |
| `selector/modhash.rs:4` | `use crate::{Endpoint, Result, TarsError};` | `use tars_protocol::Endpoint;\nuse tars_core::{Result, TarsError};` |
| `selector/modhash.rs:77` (test) | `use crate::selector::{DefaultMessage, HashType};` | 保持(同 crate) |
| `selector/roundrobin.rs:5` | `use crate::{Endpoint, Result, TarsError};` | `use tars_protocol::Endpoint;\nuse tars_core::{Result, TarsError};` |
| `selector/roundrobin.rs:80` (test) | `use crate::selector::DefaultMessage;` | 保持 |
| `selector/weight.rs:3-4` | `use crate::Endpoint;\nuse crate::endpoint::WeightType;` | `use tars_protocol::{Endpoint, endpoint::WeightType};` |
| `selector/consistenthash.rs:6` | `use crate::{Endpoint, Result, TarsError, consts};` | `use tars_protocol::Endpoint;\nuse tars_core::{Result, TarsError, consts};` |
| `selector/consistenthash.rs:154` (test) | `use crate::selector::{DefaultMessage, HashType};` | 保持 |

- [ ] **Step 6: 验证**

```bash
cargo build -p tars-util
cargo test -p tars-util
```
Expected:构建通过,所有 util/selector 单元测试通过。

---

## Task 6: 创建 `tars-transport`(transport / filter)

**Files:**
- Create: `crates/tars-transport/Cargo.toml`
- Create: `crates/tars-transport/src/lib.rs`
- Move: `src/transport/` → `crates/tars-transport/src/transport/`
- Move: `src/filter/` → `crates/tars-transport/src/filter/`

- [ ] **Step 1: 创建目录与 manifest**

```bash
mkdir -p crates/tars-transport/src
```

`crates/tars-transport/Cargo.toml`:
```toml
[package]
name = "tars-transport"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Tars TCP/UDP/TLS transports and filter middleware"

[dependencies]
tars-core.workspace = true
tars-protocol.workspace = true
tars-util.workspace = true
tokio.workspace = true
tokio-rustls.workspace = true
rustls.workspace = true
rustls-pemfile.workspace = true
webpki-roots.workspace = true
bytes.workspace = true
parking_lot.workspace = true
dashmap.workspace = true
once_cell.workspace = true
async-trait.workspace = true
tracing.workspace = true
```

- [ ] **Step 2: 移动**

```bash
git mv src/transport crates/tars-transport/src/transport
git mv src/filter crates/tars-transport/src/filter
```

- [ ] **Step 3: 写 `crates/tars-transport/src/lib.rs`**

```rust
//! # tars-transport
//!
//! Network transports (TCP/UDP/TLS) and request/response filter middleware.

pub mod filter;
pub mod transport;

pub use filter::{ClientFilter, ClientFilterMiddleware, Filters, Message, ServerFilter, ServerFilterMiddleware};
pub use transport::{
    AsyncSimpleTarsClient, ClientProtocol, ServerProtocolHandler, TarsClient, TarsClientConfig,
    TarsServer, TarsServerConfig,
};
```

(具体名字按各模块 `pub use` 列表补齐。)

- [ ] **Step 4: 改写 `transport/*.rs` 引用**

| 文件:行 | 原 | 新 |
|---|---|---|
| `transport/client.rs:13-14` | `use crate::{Result, TarsError};\nuse crate::codec::PackageStatus;` | `use tars_core::{Result, TarsError};\nuse tars_core::codec::PackageStatus;` |
| `transport/server.rs:13-15` | `use crate::{Result, TarsError};\nuse crate::codec::PackageStatus;\nuse crate::util::Context;` | `use tars_core::{Result, TarsError, Context};\nuse tars_core::codec::PackageStatus;` |
| `transport/mod.rs:29` | `use crate::codec::PackageStatus;` | `use tars_core::codec::PackageStatus;` |
| `transport/mod.rs:48,57` | `crate::util::Context` | `tars_core::Context` |
| `transport/simple_client.rs:9-10` | `use crate::{Result, TarsError};\nuse crate::protocol::{RequestPacket, ResponsePacket};` | `use tars_core::{Result, TarsError};\nuse tars_protocol::{RequestPacket, ResponsePacket};` |
| `transport/tls.rs:19` | `use crate::{Result, TarsError};` | `use tars_core::{Result, TarsError};` |

- [ ] **Step 5: 改写 `filter/mod.rs` 引用**

| 文件:行 | 原 | 新 |
|---|---|---|
| `filter/mod.rs:10-12` | `use crate::Result;\nuse crate::protocol::{RequestPacket, ResponsePacket};\nuse crate::util::Context;` | `use tars_core::{Result, Context};\nuse tars_protocol::{RequestPacket, ResponsePacket};` |
| `filter/mod.rs:29,49,74,79` | `crate::selector::HashType` / `crate::selector::Message` | `tars_util::selector::HashType` / `tars_util::selector::Message` |

注意:`filter` 在新 crate 中**没有**直接依赖 `tars-util`(它在 transport 旁边)。需要在 `crates/tars-transport/Cargo.toml` 已有的 deps 中确认 `tars-util.workspace = true`(Step 1 已加)。

- [ ] **Step 6: 验证**

```bash
cargo build -p tars-transport
cargo test -p tars-transport
```
Expected:构建通过,测试通过。

---

## Task 7: 创建 `tars-registry`(registry / adapter / logger / stat)

**Files:**
- Create: `crates/tars-registry/Cargo.toml`
- Create: `crates/tars-registry/src/lib.rs`
- Move: `src/registry/`、`src/adapter/`、`src/logger/`、`src/stat/` → `crates/tars-registry/src/<name>/`

- [ ] **Step 1: 创建目录与 manifest**

```bash
mkdir -p crates/tars-registry/src
```

`crates/tars-registry/Cargo.toml`:
```toml
[package]
name = "tars-registry"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Service registry, adapter, remote logger, and stat reporting"

[dependencies]
tars-core.workspace = true
tars-protocol.workspace = true
tars-transport.workspace = true
tars-util.workspace = true
tokio.workspace = true
async-trait.workspace = true
parking_lot.workspace = true
dashmap.workspace = true
once_cell.workspace = true
chrono.workspace = true
tracing.workspace = true
crossbeam.workspace = true
```

- [ ] **Step 2: 移动**

```bash
git mv src/registry crates/tars-registry/src/registry
git mv src/adapter crates/tars-registry/src/adapter
git mv src/logger crates/tars-registry/src/logger
git mv src/stat crates/tars-registry/src/stat
```

- [ ] **Step 3: 写 `crates/tars-registry/src/lib.rs`**

```rust
//! # tars-registry
//!
//! Service registry/adapter and remote logging/stat reporting.

pub mod adapter;
pub mod logger;
pub mod registry;
pub mod stat;

pub use adapter::AdapterProxy;
pub use logger::{LogLevel, RemoteLogConfig, RemoteTimeWriter, TarsLogger};
pub use registry::{
    DirectRegistrar, EndpointManager, NodeCircuitBreaker, Registrar, RegistryCircuitBreaker,
    TarsRegistry,
};
pub use stat::{CallTimer, StatConfig, StatReporter};
```

- [ ] **Step 4: 改写引用**

| 文件:行 | 原 | 新 |
|---|---|---|
| `adapter/mod.rs:11` | `use crate::{Endpoint, Result};` | `use tars_protocol::Endpoint;\nuse tars_core::Result;` |
| `adapter/mod.rs:12` | `use crate::protocol::{...};` | `use tars_protocol::protocol::{RequestPacket, ResponsePacket, Protocol, TarsProtocol};` |
| `adapter/mod.rs:13` | `use crate::transport::{...};` | `use tars_transport::transport::{TarsClient, TarsClientConfig, ClientProtocol};` |
| `adapter/mod.rs:14` | `use crate::codec::PackageStatus;` | `use tars_core::codec::PackageStatus;` |
| `adapter/mod.rs:15` | `use crate::consts;` | `use tars_core::consts;` |
| `adapter/mod.rs:250` | `crate::codec::parse_package(buff)` | `tars_core::codec::parse_package(buff)` |
| `logger/mod.rs:10-13` | 4 行 `use crate::...` | 替换:`use tars_protocol::protocol::logf::{...}; use tars_protocol::protocol::RequestPacket; use tars_core::codec::Buffer; use tars_transport::transport::AsyncSimpleTarsClient;` |
| `logger/mod.rs:191` | `crate::consts::TARS_ONEWAY` | `tars_core::consts::TARS_ONEWAY` |
| `stat/mod.rs:11-16` | `use crate::protocol::statf::{...};\nuse crate::protocol::RequestPacket;\nuse crate::codec::Buffer;\nuse crate::transport::AsyncSimpleTarsClient;` | 替换为 `tars_protocol::protocol::statf::{...}; tars_protocol::protocol::RequestPacket; tars_core::codec::Buffer; tars_transport::transport::AsyncSimpleTarsClient;` |
| `registry/mod.rs:20-28` | 多个 `use crate::...` | 替换:`use tars_protocol::protocol::queryf::{...}; use tars_protocol::protocol::RequestPacket; use tars_core::codec::{Buffer, Reader}; use tars_transport::transport::AsyncSimpleTarsClient; use tars_protocol::Endpoint; use tars_core::{Result, TarsError}; use tars_protocol::endpoint::ServantInstance;` |

- [ ] **Step 5: 验证**

```bash
cargo build -p tars-registry
cargo test -p tars-registry
```
Expected:构建通过,测试通过。

---

## Task 8: 创建 `tars-rpc`(servant / communicator / application)

**Files:**
- Create: `crates/tars-rpc/Cargo.toml`
- Create: `crates/tars-rpc/src/lib.rs`
- Move: `src/servant/`、`src/communicator/`、`src/application/` → `crates/tars-rpc/src/<name>/`

- [ ] **Step 1: 创建目录与 manifest**

```bash
mkdir -p crates/tars-rpc/src
```

`crates/tars-rpc/Cargo.toml`:
```toml
[package]
name = "tars-rpc"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Servant proxy, communicator, and application lifecycle"

[dependencies]
tars-core.workspace = true
tars-protocol.workspace = true
tars-transport.workspace = true
tars-util.workspace = true
tars-registry.workspace = true
tokio.workspace = true
async-trait.workspace = true
parking_lot.workspace = true
dashmap.workspace = true
once_cell.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
serde.workspace = true
toml.workspace = true
signal-hook.workspace = true
signal-hook-tokio.workspace = true
```

- [ ] **Step 2: 移动**

```bash
git mv src/servant crates/tars-rpc/src/servant
git mv src/communicator crates/tars-rpc/src/communicator
git mv src/application crates/tars-rpc/src/application
```

- [ ] **Step 3: 写 `crates/tars-rpc/src/lib.rs`**

```rust
//! # tars-rpc
//!
//! Top-level RPC abstractions: servant proxy, communicator and application.

pub mod application;
pub mod communicator;
pub mod servant;

pub use application::Application;
pub use communicator::Communicator;
pub use servant::ServantProxy;
```

- [ ] **Step 4: 改写引用**

| 文件:行 | 原 | 新 |
|---|---|---|
| `servant/mod.rs:11` | `use crate::{Result, TarsError, Endpoint};` | `use tars_protocol::Endpoint;\nuse tars_core::{Result, TarsError};` |
| `servant/mod.rs:12` | `use crate::protocol::{RequestPacket, ResponsePacket, TarsProtocol};` | `use tars_protocol::protocol::{RequestPacket, ResponsePacket, TarsProtocol};` |
| `servant/mod.rs:13` | `use crate::selector::{Selector, HashType, create_selector};` | `use tars_util::selector::{Selector, HashType, create_selector};` |
| `servant/mod.rs:14` | `use crate::adapter::AdapterProxy;` | `use tars_registry::adapter::AdapterProxy;` |
| `servant/mod.rs:15` | `use crate::transport::TarsClientConfig;` | `use tars_transport::transport::TarsClientConfig;` |
| `servant/mod.rs:16` | `use crate::filter::Message;` | `use tars_transport::filter::Message;` |
| `servant/mod.rs:17` | `use crate::util::Context;` | `use tars_core::Context;` |
| `servant/mod.rs:18` | `use crate::consts;` | `use tars_core::consts;` |
| `communicator/mod.rs:10` | `use crate::{Result, TarsError, Endpoint};` | `use tars_protocol::Endpoint;\nuse tars_core::{Result, TarsError};` |
| `communicator/mod.rs:11` | `use crate::servant::ServantProxy;` | `use crate::servant::ServantProxy;`(同 crate,保持) |
| `communicator/mod.rs:12` | `use crate::transport::TarsClientConfig;` | `use tars_transport::transport::TarsClientConfig;` |
| `communicator/mod.rs:13`(Task 1 后) | `use crate::util::ClientConfig;\nuse crate::endpoint::parse_obj_name;` | `use tars_util::ClientConfig;\nuse tars_protocol::endpoint::parse_obj_name;` |
| `application/mod.rs:12-16` | 5 行 `use crate::...` | 替换:`use tars_core::Result; use tars_transport::transport::{TarsServer, TarsServerConfig, ServerProtocolHandler}; use tars_util::{ServerConfig, ClientConfig}; use crate::communicator::Communicator; use tars_transport::filter::Filters;` |
| `application/mod.rs:203,208` | `crate::filter::ClientFilterMiddleware` / `crate::filter::ServerFilterMiddleware` | `tars_transport::filter::ClientFilterMiddleware` / `tars_transport::filter::ServerFilterMiddleware` |

- [ ] **Step 5: 验证**

```bash
cargo build -p tars-rpc
cargo test -p tars-rpc
```
Expected:构建通过,测试通过。

---

## Task 9: 创建 facade `tars` crate

**Files:**
- Create: `crates/tars/Cargo.toml`
- Create: `crates/tars/src/lib.rs`
- Move: `src/lib.rs` → `crates/tars/src/lib.rs`(整体重写)
- Move: `examples/client.rs` → `crates/tars/examples/client.rs`
- Delete: 空的 `src/` 目录

- [ ] **Step 1: 创建目录与 manifest**

```bash
mkdir -p crates/tars/src crates/tars/examples
```

`crates/tars/Cargo.toml`:
```toml
[package]
name = "tars"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Tars RPC Framework for Rust - A high-performance microservice communication framework"
keywords = ["rpc", "microservice", "tars", "distributed"]
categories = ["network-programming", "asynchronous"]

[dependencies]
tars-core.workspace = true
tars-protocol.workspace = true
tars-util.workspace = true
tars-transport.workspace = true
tars-registry.workspace = true
tars-rpc.workspace = true

[dev-dependencies]
tokio.workspace = true
```

- [ ] **Step 2: 移动 example 与删除旧 lib.rs**

```bash
git mv examples/client.rs crates/tars/examples/client.rs
git rm src/lib.rs
```

- [ ] **Step 3: 写新的 `crates/tars/src/lib.rs`(facade re-exports)**

```rust
//! # Tars RPC Framework for Rust
//!
//! Facade re-exporting the public API from `tars-core`, `tars-protocol`,
//! `tars-util`, `tars-transport`, `tars-registry`, and `tars-rpc`.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use tars::{Application, Communicator};
//!
//! #[tokio::main]
//! async fn main() {
//!     let comm = Communicator::new();
//!     // ...
//! }
//! ```

// Sub-module re-exports (preserve `tars::<module>::...` paths)
pub use tars_core::{codec, consts, error, Context};
pub use tars_protocol::{endpoint, protocol};
pub use tars_util::{selector, util};
pub use tars_transport::{filter, transport};
pub use tars_registry::{adapter, logger, registry, stat};
pub use tars_rpc::{application, communicator, servant};

// Flat re-exports (preserve `tars::Foo` paths from old lib.rs)
pub use tars_core::error::{Result, TarsError};
pub use tars_core::codec::{Buffer, Reader};
pub use tars_protocol::endpoint::Endpoint;
pub use tars_protocol::protocol::{
    EndpointF, LogInfo, PacketType, RequestPacket, ResponsePacket, StatInfo, StatMicMsgBody,
    StatMicMsgHead, TarsVersion,
};
pub use tars_util::selector::{HashType, Selector};
pub use tars_transport::filter::{
    ClientFilter, ClientFilterMiddleware, ServerFilter, ServerFilterMiddleware,
};
pub use tars_transport::transport::{
    TarsClient, TarsClientConfig, TarsServer, TarsServerConfig,
};
pub use tars_registry::adapter::AdapterProxy;
pub use tars_registry::logger::{LogLevel, RemoteLogConfig, RemoteTimeWriter, TarsLogger};
pub use tars_registry::registry::{
    DirectRegistrar, EndpointManager, NodeCircuitBreaker, Registrar, RegistryCircuitBreaker,
    TarsRegistry,
};
pub use tars_registry::stat::{CallTimer, StatConfig, StatReporter};
pub use tars_rpc::application::Application;
pub use tars_rpc::communicator::Communicator;
pub use tars_rpc::servant::ServantProxy;
```

- [ ] **Step 4: 删除已空的 `src/` 目录**

```bash
rmdir src
```
Expected:成功(目录应已为空)。如果非空,先 `ls src/` 找到残留文件,搬到合适的 crate 后重试。

- [ ] **Step 5: 验证 facade crate**

```bash
cargo build -p tars
```
Expected:成功。

- [ ] **Step 6: 验证 example**

```bash
cargo build -p tars --example client
```
Expected:成功(`client.rs` 自身不依赖 `tars` 库,但仍随 tars 包一起编译)。

---

## Task 10: 全量构建与测试

**Files:**(无)

- [ ] **Step 1: 全 workspace 构建**

```bash
cargo build --workspace
```
Expected:成功,无错误。

- [ ] **Step 2: 全 workspace 测试**

```bash
cargo test --workspace
```
Expected:测试总数 = 110 passed / 3 ignored(与 baseline 一致)。如果数量不匹配,定位丢失测试并修复。

- [ ] **Step 3: 验证 example 可执行**

```bash
cargo run -p tars --example client --no-run
```
(或仅 build:`cargo build -p tars --example client`)
Expected:成功编译。

- [ ] **Step 4: 验证 facade 公开 API 完整**

```bash
cargo doc -p tars --no-deps 2>&1 | grep -E "warning|error" | head -20
```
Expected:无 broken intra-doc link、无 missing-export 类警告。

---

## Task 11: 清理与提交

**Files:**
- Delete: 残留空目录(若有)

- [ ] **Step 1: 检查仓库状态**

```bash
git status
ls -la src/ 2>&1 || echo "src directory removed (expected)"
ls examples/
```
Expected:`src/` 不存在;`examples/` 仅剩 `hello/` 子目录。

- [ ] **Step 2: 检查 Cargo.lock**

```bash
git diff Cargo.lock | head -40
```
Expected:Cargo.lock 应反映新加入的内部 crate 名(`tars-core`、`tars-protocol` 等)。

- [ ] **Step 3: 一次性 commit 整个 workspace 拆分**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor: split tars-rs into a 7-member cargo workspace

Splits the monolithic `tars` crate into:
  - tars-core      (error, consts, codec, Context)
  - tars-protocol  (protocol packets, endpoint)
  - tars-util      (configs, request id, selectors)
  - tars-transport (TCP/UDP/TLS transports, filter middleware)
  - tars-registry  (registry, adapter, remote logger, stat)
  - tars-rpc       (servant, communicator, application)
  - tars           (facade re-exporting the existing public API)

The `tars` crate keeps its name and re-export surface so external
users' `use tars::xxx` paths are unchanged.

Endpoint parsing helpers (`parse_endpoint_string`, `parse_obj_name`)
are moved from `util` into `endpoint` to break the now-cross-crate
cycle. Context is hoisted into `tars-core` so both protocol-layer
and transport-layer code can reference it without depending on the
util crate.

Drops the unused `tls`/`opentelemetry` empty features and the unused
`tokio-test` dev-dep.
EOF
)"
```

- [ ] **Step 4: 最后一次完整验证**

```bash
cargo clean
cargo build --workspace
cargo test --workspace
```
Expected:从零编译成功;110 passed / 3 ignored。

---

## Self-Review Notes

下面这些点我在写计划时核对过,贴出来供执行时校验:

1. **Spec 覆盖检查**
   - ✅ Workspace 7 个成员 — Task 2(根 manifest)+ Task 3-9(每个成员一个)
   - ✅ Facade re-exports 完整 — Task 9 Step 3 列出全部
   - ✅ `tls = []` / `opentelemetry = []` / `tokio-test` 删除 — Task 2 Step 1 中根 Cargo.toml 重写时省略
   - ✅ Examples 处理 — Task 9 Step 2(client.rs 移走),`examples/hello/` 不动
   - ✅ Workspace 公共配置 — Task 2 Step 1 已包含 `[workspace.package]` 和 `[workspace.dependencies]`

2. **拆分粒度对照 spec 表格**

| spec 行 | 计划 Task |
|---|---|
| `tars-core` = error/consts/codec | Task 3(并把 Context 也提到这里 — spec 风险章节"如果发现新增循环,优先把循环里的最小公共类型上提到更底层 crate"已预先批准了这种调整) |
| `tars-protocol` = protocol/endpoint | Task 4 |
| `tars-util` = util/selector | Task 5 |
| `tars-transport` = transport/filter | Task 6 |
| `tars-registry` = registry/adapter/logger/stat | Task 7 |
| `tars-rpc` = servant/communicator/application | Task 8 |
| `tars` = facade | Task 9 |

3. **依赖关系不形成环**
   - tars-core ← tars-protocol ← tars-util ← tars-transport ← tars-registry ← tars-rpc ← tars
   - filter(tars-transport)使用 selector(tars-util):tars-transport 依赖 tars-util ✓
   - 提前(Task 1)消除 endpoint→util 循环;Task 4 Step 4 提前迁移 Context 到 tars-core 解决 protocol→util 隐式依赖

4. **Placeholder 扫描** — 全文无 TBD/TODO/“implement later”。所有代码块给出完整内容,所有命令给出预期输出。

5. **类型一致性** — facade `tars::Foo` 路径与原 `src/lib.rs:46-60` 的 `pub use` 列表一一对应(Task 9 Step 3 已逐项匹配)。

---

## 执行入口

实施时建议使用 **superpowers:executing-plans**(本会话内顺序执行,每完成 1-2 个 task 做一次检查点),因为:
- 任务间高度顺序依赖(下游 crate 依赖上游 crate 已就位)
- 中途 build 状态有意被打破(Task 2 之后到 Task 9 之间根目录无法独立 build),subagent 之间隔离过强反而难以共享上下文
- 每个 task 都有 `cargo build -p` 即时反馈
