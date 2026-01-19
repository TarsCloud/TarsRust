# TarsRust

[中文文档](./README_zh.md)

A high-performance RPC framework for Rust, compatible with the [TARS](https://github.com/TarsCloud/Tars) ecosystem.

## Overview

TarsRust is the Rust implementation of the TARS RPC framework, providing the same functionality as [TarsGo](https://github.com/TarsCloud/TarsGo) and [TarsCpp](https://github.com/TarsCloud/TarsCpp). It enables Rust applications to seamlessly integrate with TARS microservices infrastructure.

## Features

- **High Performance**: Built on Tokio async runtime for excellent concurrency
- **Protocol Compatible**: Full compatibility with TARS protocol (v1/v2/v3)
- **Load Balancing**: Multiple strategies including Round Robin, Random, Mod Hash, and Consistent Hash
- **Service Discovery**: Integration with TARS registry for automatic service discovery
- **Transport Layer**: Support for TCP, UDP, and TLS/SSL
- **Fault Tolerance**: Built-in health checking, connection pooling, and automatic reconnection
- **Observability**: Structured logging with tracing, statistics reporting

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
tars = { git = "https://github.com/TarsCloud/TarsRust" }
tokio = { version = "1.35", features = ["full"] }
```

## Quick Start

### Client Example

```rust
use tars::{Communicator, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a communicator
    let comm = Communicator::new();

    // Create a servant proxy with direct endpoint
    let proxy = comm.string_to_proxy(
        "Hello.HelloServer.HelloObj@tcp -h 127.0.0.1 -p 18015"
    )?;

    // Set timeout (milliseconds)
    proxy.set_timeout(3000);

    // Make RPC call (you need to encode request according to your .tars interface)
    let request_data = encode_request("sayHello", "World");
    let response = proxy.invoke("sayHello", &request_data).await?;

    println!("Response: {:?}", response);
    Ok(())
}
```

### Server Example

Define your interface in a `.tars` file:

```tars
// Hello.tars
module Hello {
    interface HelloWorld {
        int sayHello(string name, out string greeting);
    };
};
```

Implement the service:

```rust
use tars::{Application, Result};

// Implement the HelloWorld interface
struct HelloWorldImp;

impl HelloWorldImp {
    // sayHello returns a greeting message
    fn say_hello(&self, name: &str) -> (i32, String) {
        let greeting = format!("Hello, {}! Welcome to Tars.", name);
        (0, greeting)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get server config
    let cfg = Application::get_server_config();
    println!("Starting server: {}.{}", cfg.app, cfg.server);

    // Create service implementation
    let imp = HelloWorldImp;

    // Register servant and start server
    let app = Application::new();
    app.add_servant("Hello.HelloServer.HelloWorldObj", imp)?;

    println!("Server is running...");
    app.run().await
}
```

## Architecture

TarsRust is organized into the following layers:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│         (Application lifecycle, configuration)               │
├─────────────────────────────────────────────────────────────┤
│                      Proxy Layer                             │
│      (ServantProxy, Communicator, Filters)                   │
├─────────────────────────────────────────────────────────────┤
│                    Endpoint Layer                            │
│   (Endpoint management, Load balancing, Health check)        │
├─────────────────────────────────────────────────────────────┤
│                    Transport Layer                           │
│         (TCP/UDP/TLS connection management)                  │
├─────────────────────────────────────────────────────────────┤
│                    Protocol Layer                            │
│           (TARS protocol encoding/decoding)                  │
└─────────────────────────────────────────────────────────────┘
```

## Modules

| Module | Description |
|--------|-------------|
| `codec` | TLV encoding/decoding for TARS protocol |
| `protocol` | Request/Response packet structures |
| `endpoint` | Endpoint definition and management |
| `selector` | Load balancing strategies |
| `transport` | Client and server transport implementations |
| `registry` | Service discovery and registration |
| `communicator` | Client-side communication management |
| `application` | Application lifecycle management |
| `filter` | Client and server filter middleware |
| `logger` | Remote logging support |
| `stat` | Statistics reporting |

## Load Balancing

TarsRust supports multiple load balancing strategies:

| Strategy | Description | Use Case |
|----------|-------------|----------|
| Round Robin | Cycles through endpoints in order | Default, general purpose |
| Random | Randomly selects an endpoint | Even distribution |
| Mod Hash | Selects based on hash % node_count | Session affinity |
| Consistent Hash | Virtual nodes with consistent hashing | Minimizes redistribution on changes |

### Example: Using Hash-based Routing

```rust
use tars::selector::{HashType, DefaultMessage};

// Create a message with hash routing
let msg = DefaultMessage::with_hash(user_id.hash(), HashType::ConsistentHash);
let endpoint = selector.select(&msg)?;
```

## Configuration

### Client Configuration

```rust
use tars::transport::TarsClientConfig;
use std::time::Duration;

let config = TarsClientConfig::tcp()
    .with_queue_len(10000)
    .with_idle_timeout(Duration::from_secs(600))
    .with_read_timeout(Duration::from_secs(30))
    .with_write_timeout(Duration::from_secs(30))
    .with_dial_timeout(Duration::from_secs(3));
```

### Server Configuration

```rust
use tars::transport::TarsServerConfig;
use std::time::Duration;

let config = TarsServerConfig::tcp("0.0.0.0:18015")
    .with_max_invoke(200000)
    .with_read_timeout(Duration::from_secs(30))
    .with_write_timeout(Duration::from_secs(30))
    .with_handle_timeout(Duration::from_secs(60))
    .with_idle_timeout(Duration::from_secs(600));
```

## Working with TARS Go Server

TarsRust can communicate with any TARS server. Here's an example of calling a TarsGo server:

### 1. Define Interface (Hello.tars)

```tars
module Hello {
    interface HelloWorld {
        int sayHello(string name, out string greeting);
    };
};
```

### 2. Go Server Implementation

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

### 3. Rust Client

```rust
use tars::{Communicator, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Create communicator
    let comm = Communicator::new();

    // Create proxy to the Go server
    let proxy = comm.string_to_proxy(
        "Hello.HelloServer.HelloWorldObj@tcp -h 127.0.0.1 -p 18015"
    )?;

    // Call sayHello method
    let name = "Rust Client";
    let (ret, greeting) = proxy.say_hello(name).await?;

    println!("Return: {}", ret);
    println!("Greeting: {}", greeting);

    Ok(())
}
```

## Error Handling

TarsRust provides comprehensive error types:

```rust
use tars::{TarsError, Result};

fn handle_error(result: Result<()>) {
    match result {
        Ok(_) => println!("Success"),
        Err(TarsError::Timeout(ms)) => println!("Request timed out after {}ms", ms),
        Err(TarsError::NoEndpoint) => println!("No available endpoint"),
        Err(TarsError::ServiceNotFound(name)) => println!("Service not found: {}", name),
        Err(TarsError::ServerError { code, message }) => {
            println!("Server error: code={}, message={}", code, message)
        }
        Err(e) => println!("Other error: {}", e),
    }
}
```

## Transport Protocols

| Protocol | Description | Use Case |
|----------|-------------|----------|
| TCP | Reliable, ordered delivery | Default, most use cases |
| UDP | Fast, connectionless | Low latency, can tolerate loss |
| SSL/TLS | Encrypted TCP | Security-sensitive communications |

### Creating Endpoints

```rust
use tars::Endpoint;

// TCP endpoint
let tcp_ep = Endpoint::tcp("127.0.0.1", 10000);

// UDP endpoint
let udp_ep = Endpoint::udp("127.0.0.1", 10001);

// SSL endpoint
let ssl_ep = Endpoint::ssl("127.0.0.1", 10002);

// Parse from string
let ep = Endpoint::from_string("tcp -h 127.0.0.1 -p 10000 -t 3000");
```

## Constants

Important protocol constants:

```rust
use tars::consts;

// Protocol versions
const TARS_VERSION: i16 = consts::TARS_VERSION;    // 1
const TUP_VERSION: i16 = consts::TUP_VERSION;      // 2
const JSON_VERSION: i16 = consts::JSON_VERSION;    // 3

// Return codes
const SUCCESS: i32 = consts::TARS_SERVER_SUCCESS;           // 0
const DECODE_ERR: i32 = consts::TARS_SERVER_DECODE_ERR;     // -1
const QUEUE_TIMEOUT: i32 = consts::TARS_SERVER_QUEUE_TIMEOUT; // -2
const INVOKE_TIMEOUT: i32 = consts::TARS_INVOKE_TIMEOUT;    // -3

// Default timeouts (ms)
const ASYNC_TIMEOUT: u64 = consts::DEFAULT_ASYNC_TIMEOUT;   // 3000
const CONNECT_TIMEOUT: u64 = consts::DEFAULT_CONNECT_TIMEOUT; // 3000
```

## Running Examples

### Prerequisites

1. Start a TarsGo HelloWorld server:

```bash
cd examples/hello
go build -o HelloServer
./HelloServer --config HelloServer.conf
```

2. Run the Rust client:

```bash
cargo run --example client
```

### Expected Output

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

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

This project is licensed under the BSD-3-Clause License - see the [LICENSE](LICENSE) file for details.

## Related Projects

- [TARS](https://github.com/TarsCloud/Tars) - TARS Framework
- [TarsGo](https://github.com/TarsCloud/TarsGo) - Go implementation
- [TarsCpp](https://github.com/TarsCloud/TarsCpp) - C++ implementation
- [TarsJava](https://github.com/TarsCloud/TarsJava) - Java implementation
- [TarsPHP](https://github.com/TarsCloud/TarsPHP) - PHP implementation
