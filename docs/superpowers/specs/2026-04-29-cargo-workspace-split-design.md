# Cargo Workspace 拆分设计

**日期:** 2026-04-29
**状态:** 已批准

## 背景

当前 `tars-rs` 是单 crate 项目,`src/` 下有 14 个子模块,约 9000 行代码。所有模块都通过 `crate::xxx` 互相耦合在一个编译单元里,改任何一处都会触发全量重编译,且 `tars-core`、`tars-protocol` 这类底层稳定模块无法被独立复用。

本次重构将其拆分为 Cargo workspace,按职责边界划分为 7 个 crate,层次依赖清晰。

## 目标与非目标

**目标:**

- 按职责切分为多个 crate,使底层(codec/protocol)独立编译
- 保持外部 API 完全兼容:`use tars::xxx` 的路径不变
- 统一管理第三方依赖版本
- 拆分后 `cargo build --workspace` / `cargo test --workspace` / `cargo run --example client` 均通过

**非目标:**

- 不调整模块的内部实现逻辑
- 不新增功能、不重命名公开 API
- 不引入新 feature flag(同时清理已废弃的空 feature)

## Workspace 布局

```
tars-rs/
├── Cargo.toml                # virtual workspace manifest
├── crates/
│   ├── tars-core/            # error / Result / consts + codec
│   ├── tars-protocol/        # protocol + endpoint
│   ├── tars-util/            # util + selector
│   ├── tars-transport/       # transport + filter
│   ├── tars-registry/        # registry + adapter + logger + stat
│   ├── tars-rpc/             # servant + communicator + application
│   └── tars/                 # facade,re-export 全部公开 API
│       └── examples/
│           └── client.rs     # Rust example 随 tars crate 一起
└── examples/
    └── hello/                # 原 Go demo(非 Rust),保留原位
```

## Crate 划分与依赖

| Crate | 包含的 src 模块 | 依赖的工作区成员 |
|---|---|---|
| `tars-core` | `error`、`consts`(原 `lib.rs`)、`codec` | — |
| `tars-protocol` | `protocol`、`endpoint` | tars-core |
| `tars-util` | `util`、`selector` | tars-core, tars-protocol |
| `tars-transport` | `transport`、`filter` | tars-core, tars-protocol, tars-util |
| `tars-registry` | `registry`、`adapter`、`logger`、`stat` | tars-core, tars-protocol, tars-transport, tars-util |
| `tars-rpc` | `servant`、`communicator`、`application` | tars-core, tars-protocol, tars-transport, tars-util, tars-registry |
| `tars` | 仅 `lib.rs`(re-export) | 上述全部 |

依赖图为单向无环,层级:`core → protocol → util → transport → registry → rpc → facade`。

### 各 crate 的依赖来源(基于现有代码扫描)

- **tars-core**:codec 现仅使用 `Result/TarsError`,把 error/consts/codec 放在一起即自给自足。
- **tars-protocol**:protocol 当前 `use crate::codec::{Buffer, Reader}`、`use crate::Result`;endpoint `use crate::protocol::TransportProtocol`。两者只依赖 core。
- **tars-util**:util 当前 `use crate::endpoint::Endpoint` / `use crate::protocol::TransportProtocol`;selector `use crate::Endpoint` / `use crate::endpoint::WeightType`。合并到一个 crate 后只依赖 core + protocol。
- **tars-transport**:transport `use crate::codec::PackageStatus` / `use crate::protocol::*` / `use crate::util::Context`;filter `use crate::protocol::*` / `use crate::util::Context`。
- **tars-registry**:registry/adapter/logger/stat 都用到 codec、protocol、transport,registry/adapter 还用到 endpoint。
- **tars-rpc**:servant `use crate::adapter::*` / `use crate::filter::*` / `use crate::selector::*` / `use crate::transport::*`;communicator/application 在其上层。
- **tars**:对应当前的 `src/lib.rs`,所有 `pub use` 改为 `pub use tars_xxx::yyy`。

## 兼容性策略

`tars` facade crate 保留原 crate 名 `tars`,以及现有 `lib.rs` 中所有 `pub use`,使外部用户的 `use tars::Communicator` 等路径不变。

`error::TarsError`、`Result`、`consts` 在 `tars-core` 中定义,`tars` facade 通过 `pub use tars_core::error;` 和 `pub use tars_core::{TarsError, Result};` 等保持现有路径可见。

各成员 crate 内部的 `use crate::xxx` 改写为 `use tars_protocol::xxx` / `use tars_core::Result` 等。

## 公共配置

根 `Cargo.toml` 使用 virtual workspace(无顶层 `[package]`):

- `[workspace]`:`members = ["crates/*"]`
- `[workspace.package]`:统一 `version = "0.1.0"`、`edition = "2021"`、`license = "BSD-3-Clause"`、`authors`、`repository`
- `[workspace.dependencies]`:统一管理所有第三方 crate 版本(tokio、bytes、tracing、rustls 系列、serde、anyhow、thiserror 等)
- `[profile.release]`:沿用现有 LTO/codegen-units 配置

各成员 `Cargo.toml` 用 `tokio.workspace = true` 形式继承版本。

## Feature flags 清理

当前根 `Cargo.toml` 的 `tls = []` 与 `opentelemetry = []` 均为空 feature,代码中无任何 `#[cfg(feature = ...)]` 引用。本次直接删除,后续如需再独立设计。

## 验证标准

- `cargo build --workspace` 通过(无 warning 退化)
- `cargo test --workspace` 通过
- `cargo run --example client` 可执行
- `tars` crate 公共路径与拆分前完全一致(对照拆分前 `src/lib.rs` 的 `pub use` 列表)

## 风险与对策

- **循环依赖风险**:已扫描所有 `use crate::` 引用,确认按上述分组无环。如实际重构中发现新增循环,优先把循环里的最小公共类型上提到更底层的 crate(通常是 tars-core 或 tars-protocol)。
- **测试与子模块路径**:模块内的 `#[cfg(test)] mod tests { use crate::... }` 写法在迁移时需改为 `use crate::xxx`(每个 crate 内部)或 `use tars_xxx::...`。
- **Examples 路径**:`examples/client.rs` 移到 `crates/tars/examples/client.rs`,根目录 `examples/hello/`(Go demo)保留原位不动。`cargo run --example client` 需带 `-p tars` 参数,或在根 `Cargo.toml` 的 `[workspace]` 段下让默认成员包含 tars。
