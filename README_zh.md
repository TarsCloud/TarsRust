# TarsRust

[English](./README.md)

> **警告**
> 这是一个早期实验版本，API 不稳定，可能会发生重大变化。**请勿用于生产环境。**

Rust 语言实现的高性能 RPC 框架，与 [TARS](https://github.com/TarsCloud/Tars) 生态系统完全兼容。

## 概述

TarsRust 是 TARS RPC 框架的 Rust 实现，提供与 [TarsCpp](https://github.com/TarsCloud/TarsCpp) 和其他 TARS 语言实现相同的功能。它使 Rust 应用程序能够无缝集成到 TARS 微服务基础设施中。

## 特性

- **高性能**：基于 Tokio 异步运行时，提供出色的并发处理能力
- **协议兼容**：完全兼容 TARS 协议（v1/v2/v3）
- **负载均衡**：支持多种策略，包括轮询、随机、取模哈希和一致性哈希
- **服务发现**：与 TARS 注册中心集成，支持自动服务发现
- **传输层**：支持 TCP、UDP 和 TLS/SSL
- **容错机制**：内置健康检查、连接池和自动重连
- **可观测性**：结构化日志记录（tracing）和统计上报

## 安装

在 `Cargo.toml` 中添加以下依赖：

```toml
[dependencies]
tars = { git = "https://github.com/TarsCloud/TarsRust" }
tokio = { version = "1.35", features = ["full"] }
```

## 快速开始

### 客户端示例

```rust
use tars::{Communicator, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建通信器
    let comm = Communicator::new();

    // 通过直连地址创建服务代理
    let proxy = comm.string_to_proxy(
        "Hello.HelloServer.HelloObj@tcp -h 127.0.0.1 -p 18015"
    )?;

    // 设置超时时间（毫秒）
    proxy.set_timeout(3000);

    // 发起 RPC 调用（需要根据 .tars 接口定义编码请求）
    let request_data = encode_request("sayHello", "World");
    let response = proxy.invoke("sayHello", &request_data).await?;

    println!("响应: {:?}", response);
    Ok(())
}
```

### 服务端示例

在 `.tars` 文件中定义接口：

```tars
// Hello.tars
module Hello {
    interface HelloWorld {
        int sayHello(string name, out string greeting);
    };
};
```

实现服务：

```rust
use tars::{Application, Result};

// 实现 HelloWorld 接口
struct HelloWorldImp;

impl HelloWorldImp {
    // sayHello 返回问候消息
    fn say_hello(&self, name: &str) -> (i32, String) {
        let greeting = format!("Hello, {}! Welcome to Tars.", name);
        (0, greeting)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 获取服务配置
    let cfg = Application::get_server_config();
    println!("启动服务: {}.{}", cfg.app, cfg.server);

    // 创建服务实现
    let imp = HelloWorldImp;

    // 注册 servant 并启动服务
    let app = Application::new();
    app.add_servant("Hello.HelloServer.HelloWorldObj", imp)?;

    println!("服务运行中...");
    app.run().await
}
```

## 架构设计

TarsRust 采用分层架构设计：

```
┌─────────────────────────────────────────────────────────────┐
│                      应用层 (Application)                    │
│              (应用生命周期管理、配置解析)                       │
├─────────────────────────────────────────────────────────────┤
│                      代理层 (Proxy)                          │
│           (ServantProxy、Communicator、Filter)               │
├─────────────────────────────────────────────────────────────┤
│                     端点层 (Endpoint)                        │
│            (端点管理、负载均衡、健康检查)                       │
├─────────────────────────────────────────────────────────────┤
│                     传输层 (Transport)                       │
│              (TCP/UDP/TLS 连接管理)                          │
├─────────────────────────────────────────────────────────────┤
│                     协议层 (Protocol)                        │
│               (TARS 协议编解码)                              │
└─────────────────────────────────────────────────────────────┘
```

## 模块说明

| 模块 | 说明 |
|------|------|
| `codec` | TARS 协议的 TLV 编解码 |
| `protocol` | 请求/响应数据包结构定义 |
| `endpoint` | 端点定义和管理 |
| `selector` | 负载均衡策略实现 |
| `transport` | 客户端和服务端传输层实现 |
| `registry` | 服务发现和注册 |
| `communicator` | 客户端通信管理 |
| `application` | 应用生命周期管理 |
| `filter` | 客户端和服务端过滤器中间件 |
| `logger` | 远程日志支持 |
| `stat` | 统计上报 |

## 负载均衡

TarsRust 支持多种负载均衡策略：

| 策略 | 说明 | 使用场景 |
|------|------|----------|
| Round Robin（轮询） | 按顺序循环选择端点 | 默认策略，通用场景 |
| Random（随机） | 随机选择端点 | 均匀分布 |
| Mod Hash（取模哈希） | 基于 hash % 节点数选择 | 会话亲和性 |
| Consistent Hash（一致性哈希） | 使用虚拟节点的一致性哈希 | 最小化节点变化时的重分布 |

### 示例：使用哈希路由

```rust
use tars::selector::{HashType, DefaultMessage};

// 创建带哈希路由的消息
let msg = DefaultMessage::with_hash(user_id.hash(), HashType::ConsistentHash);
let endpoint = selector.select(&msg)?;
```

## 配置说明

### 客户端配置

```rust
use tars::transport::TarsClientConfig;
use std::time::Duration;

let config = TarsClientConfig::tcp()
    .with_queue_len(10000)                        // 发送队列长度
    .with_idle_timeout(Duration::from_secs(600))  // 空闲超时
    .with_read_timeout(Duration::from_secs(30))   // 读取超时
    .with_write_timeout(Duration::from_secs(30))  // 写入超时
    .with_dial_timeout(Duration::from_secs(3));   // 连接超时
```

### 服务端配置

```rust
use tars::transport::TarsServerConfig;
use std::time::Duration;

let config = TarsServerConfig::tcp("0.0.0.0:18015")
    .with_max_invoke(200000)                       // 最大并发调用数
    .with_read_timeout(Duration::from_secs(30))    // 读取超时
    .with_write_timeout(Duration::from_secs(30))   // 写入超时
    .with_handle_timeout(Duration::from_secs(60))  // 处理超时
    .with_idle_timeout(Duration::from_secs(600));  // 空闲超时
```

## 与 TARS 服务端交互

TarsRust 可以与任何 TARS 服务端通信。以下是调用 TARS 服务端的示例：

### 1. 定义接口 (Hello.tars)

```tars
module Hello {
    interface HelloWorld {
        int sayHello(string name, out string greeting);
    };
};
```

### 2. Go 服务端实现

```go
// HelloWorldImp.go
package main

type HelloWorldImp struct{}

func (h *HelloWorldImp) SayHello(name string, greeting *string) (int32, error) {
    *greeting = "Hello, " + name + "!"
    return 0, nil
}

// main.go
func main() {
    cfg := tars.GetServerConfig()
    imp := new(HelloWorldImp)
    app := new(Hello.HelloWorld)
    app.AddServant(imp, cfg.App+"."+cfg.Server+".HelloWorldObj")
    tars.Run()
}
```

### 3. Rust 客户端

```rust
use tars::{Communicator, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建通信器
    let comm = Communicator::new();

    // 创建到 Go 服务端的代理
    let proxy = comm.string_to_proxy(
        "Hello.HelloServer.HelloWorldObj@tcp -h 127.0.0.1 -p 18015"
    )?;

    // 调用 sayHello 方法
    let name = "Rust Client";
    let (ret, greeting) = proxy.say_hello(name).await?;

    println!("返回值: {}", ret);
    println!("问候语: {}", greeting);

    Ok(())
}
```

## 错误处理

TarsRust 提供完善的错误类型：

```rust
use tars::{TarsError, Result};

fn handle_error(result: Result<()>) {
    match result {
        Ok(_) => println!("成功"),
        Err(TarsError::Timeout(ms)) => println!("请求超时，耗时 {}ms", ms),
        Err(TarsError::NoEndpoint) => println!("没有可用的端点"),
        Err(TarsError::ServiceNotFound(name)) => println!("服务未找到: {}", name),
        Err(TarsError::ServerError { code, message }) => {
            println!("服务端错误: code={}, message={}", code, message)
        }
        Err(e) => println!("其他错误: {}", e),
    }
}
```

## 传输协议

| 协议 | 说明 | 使用场景 |
|------|------|----------|
| TCP | 可靠、有序的数据传输 | 默认选择，适用于大多数场景 |
| UDP | 快速、无连接的传输 | 低延迟场景，可容忍丢包 |
| SSL/TLS | 加密的 TCP 连接 | 安全敏感的通信 |

### 创建端点

```rust
use tars::Endpoint;

// TCP 端点
let tcp_ep = Endpoint::tcp("127.0.0.1", 10000);

// UDP 端点
let udp_ep = Endpoint::udp("127.0.0.1", 10001);

// SSL 端点
let ssl_ep = Endpoint::ssl("127.0.0.1", 10002);

// 从字符串解析
let ep = Endpoint::from_string("tcp -h 127.0.0.1 -p 10000 -t 3000");
```

## 协议常量

重要的协议常量定义：

```rust
use tars::consts;

// 协议版本
const TARS_VERSION: i16 = consts::TARS_VERSION;    // 1
const TUP_VERSION: i16 = consts::TUP_VERSION;      // 2
const JSON_VERSION: i16 = consts::JSON_VERSION;    // 3

// 返回码
const SUCCESS: i32 = consts::TARS_SERVER_SUCCESS;           // 0 成功
const DECODE_ERR: i32 = consts::TARS_SERVER_DECODE_ERR;     // -1 解码错误
const QUEUE_TIMEOUT: i32 = consts::TARS_SERVER_QUEUE_TIMEOUT; // -2 队列超时
const INVOKE_TIMEOUT: i32 = consts::TARS_INVOKE_TIMEOUT;    // -3 调用超时

// 默认超时（毫秒）
const ASYNC_TIMEOUT: u64 = consts::DEFAULT_ASYNC_TIMEOUT;   // 3000
const CONNECT_TIMEOUT: u64 = consts::DEFAULT_CONNECT_TIMEOUT; // 3000
```

## 运行示例

### 前置条件

1. 启动 Go 版 HelloWorld 服务端：

```bash
cd examples/hello
go build -o HelloServer
./HelloServer --config HelloServer.conf
```

2. 运行 Rust 客户端：

```bash
cargo run --example client
```

### 预期输出

```
=== Rust Tars Client Test ===

Connecting to 127.0.0.1:18015...
Connected!

Calling Hello.HelloServer.HelloWorldObj.sayHello("Rust Client")...

Sending 95 bytes request...
Received 89 bytes response

Response packet:
  Request ID: 1
  Return Code: 0
  Result Desc:
  Buffer Length: 32

Function result:
  Return: 0
  Greeting: "Hello, Rust Client!"

=== TEST PASSED ===
```

## 贡献指南

欢迎贡献代码！请随时提交 Issue 和 Pull Request。

## 许可证

本项目采用 BSD-3-Clause 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 相关项目

- [TARS](https://github.com/TarsCloud/Tars) - TARS 框架
- [Tars-Go](https://github.com/TarsCloud/TarsGo) - Go 语言实现
- [TarsCpp](https://github.com/TarsCloud/TarsCpp) - C++ 语言实现
- [TarsJava](https://github.com/TarsCloud/TarsJava) - Java 语言实现
- [TarsPHP](https://github.com/TarsCloud/TarsPHP) - PHP 语言实现
