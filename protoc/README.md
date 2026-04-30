# tarsc-go

Tars IDL Compiler written in Go — compiles `.tars` files into source code for multiple
target languages, similar to how `protoc` works for Protocol Buffers.

## Overview

`tarsc` parses Tars IDL (`.tars`) files and emits typed code (struct / enum / const /
interface) for the target language(s) you select. One invocation can generate code for
several languages at once.

## Supported Languages

| Language     | CLI name     | Aliases             | File ext  |
|--------------|--------------|---------------------|-----------|
| Go           | `go`         | `golang`            | `.go`     |
| Java         | `java`       |                     | `.java`   |
| C++          | `cpp`        | `c++`               | `.h/.cpp` |
| Python       | `python`     | `py`                | `.py`     |
| TypeScript   | `typescript` | `ts`                | `.ts`     |
| Swift        | `swift`      |                     | `.swift`  |
| Rust         | `rust`       |                     | `.rs`     |
| C#           | `csharp`     | `cs`                | `.cs`     |
| Node.js      | `node`       | `nodejs`, `js`      | `.js`     |
| PHP          | `php`        |                     | `.php`    |
| Objective-C  | `objc`       | `oc`                | `.h/.m`   |

Run `tarsc --list-langs` to print this list at runtime.

## Installation

```bash
cd tools/tarsc-go
go build -o tarsc ./cmd/tarsc
```

Requires Go 1.21+.

## Usage

```
tarsc [options] <tars files...>
```

### Options

| Flag                    | Description                                              |
|-------------------------|----------------------------------------------------------|
| `-o <lang>:<dir>`       | Target language and output directory (repeatable)        |
| `--out=<lang>:<dir>`    | Long form of `-o`                                        |
| `-I <path>`             | Include search paths (semicolon-separated)               |
| `-v`                    | Verbose mode (show parse stats and per-step progress)    |
| `--no-color`            | Disable ANSI colors (also honored via `NO_COLOR` env var)|
| `--list-langs`          | Print the supported-languages table                      |
| `--version`             | Print version info                                       |
| `--help`                | Print usage                                              |

### Examples

Generate Rust code:

```bash
tarsc -o rust:./gen/rust hello.tars
```

Generate multiple languages in one pass:

```bash
tarsc -o go:./gen/go -o java:./gen/java -o ts:./gen/ts message.tars
```

Use include paths and verbose output:

```bash
tarsc -v -I ./protocols -o cpp:./gen/cpp service.tars
```

Process multiple files:

```bash
tarsc -o java:./gen *.tars
```

## Tars IDL Syntax

### Primitive Types

`bool`, `byte` / `char`, `short`, `int`, `long`, `float`, `double`, `string`.
The `unsigned` modifier is supported for `byte`, `short`, `int`.

### Container Types

- `vector<T>` — dynamic array
- `map<K, V>` — key-value map

### User-Defined Types

```tars
module MyModule
{
    enum Status
    {
        OK = 0,
        ERROR = 1
    };

    const int MAX_SIZE = 1024;

    struct User
    {
        0 require int    id;
        1 require string name;
        2 optional int   age   = 0;
        3 optional Status status = OK;
    };

    interface UserService
    {
        User getUser(int id);
        void createUser(User user);
        int  update(int id, User user, out User result);
    };
};
```

### Field Modifiers

- `require` — field must always be encoded/decoded
- `optional` — field may be absent; default value supported
- `out` — output parameter (interface methods only)

## What Each Generator Produces

All generators produce type definitions (struct / enum / const). On top of that:

| Language     | Client (Prx) | Server (Servant) | Async Callback | Notes                                              |
|--------------|:------------:|:----------------:|:--------------:|----------------------------------------------------|
| Go           | ✓            | ✓                | ✓              | One `.go` file per namespace                       |
| Java         | ✓            | ✓                | ✓              | Separate `.java` file per type                     |
| C++          | ✓            | ✓                | ✓              | `.h` + `.cpp` pair                                  |
| Python       | ✓            | ✓                | ✓              | Single module file                                 |
| TypeScript   | ✓            | ✓                | —              | Splits into `types.ts`, `client.ts`, `server.ts`, `index.ts` |
| Swift        | ✓            | ✓                | —              | Async/await based                                  |
| Rust         | stub         | —                | —              | See [Rust generator status](#rust-generator-status)|
| C#           | ✓            | ✓                | ✓              | Single `.cs` file                                  |
| Node.js      | ✓            | ✓                | —              | Single `.js` file                                  |
| PHP          | ✓            | ✓                | —              | Single `.php` file                                 |
| Objective-C  | ✓            | ✓                | —              | `.h` + `.m` pair                                   |

### Rust generator status

The Rust backend currently emits:

- `pub struct` for each Tars struct, with `Debug`, `Clone`, `Default`, `Serialize`,
  `Deserialize` derives, plus a `TarsStruct` impl with `encode` / `decode` methods that
  use tag-based reads/writes (`encoder.write(tag, &self.field)` /
  `decoder.read_optional(tag)`).
- `pub enum` for each Tars enum, with `#[repr(i32)]`, `Default`, and `From<i32>` impls.
- `pub const` for each Tars const.
- A `pub trait` for each interface plus a `<Name>Proxy` struct that implements the
  trait. **The proxy method bodies are currently `unimplemented!()` stubs.** No
  server-side Servant code is generated.

A Rust runtime for the TARS RPC framework exists at
[**TarsCloud/TarsRust**](https://github.com/TarsCloud/TarsRust) (crate name: `tars`,
Tokio-based, exposing `Communicator` / `ServantProxy` / `proxy.invoke(name, &buf).await`).
The runtime upstream is marked early-experimental; it is suitable for development but
not production yet.

The generator is **not yet retargeted to TarsRust** — generated code currently imports
a placeholder `tars_stream` crate and uses sync `Result<T, Box<dyn Error>>` signatures
that don't match TarsRust's async API. To use the generated code today you have two
options:

1. **Use it as a typed scaffold**: keep the generated `struct` / `enum` / `const` /
   `trait` definitions and hand-write the proxy bodies against
   [`tars::ServantProxy`](https://github.com/TarsCloud/TarsRust).
2. **Wait for / contribute the codegen update** that swaps the imports to the `tars`
   crate, makes trait methods `async fn`, and emits proxy bodies along the lines of:

   ```rust
   let mut os = TarsOutputStream::new();
   os.write(&arg, 1)?;
   let resp = self.proxy.invoke("methodName", &os.into_buffer()).await?;
   let mut is = TarsInputStream::new(&resp);
   is.read(0, true)
   ```

The relevant codegen entry point is
[`pkg/generator/rust/generator.go`](pkg/generator/rust/generator.go) —
`generateProxyMethod` (proxy body) and the imports in `generateNamespace`.

## Project Structure

```
tarsc-go/
├── cmd/tarsc/main.go        # CLI entry point
├── pkg/
│   ├── ast/                 # AST node definitions
│   ├── lexer/               # Lexical analyzer
│   ├── parser/              # Parser (.tars → AST)
│   └── generator/
│       ├── cpp/
│       ├── csharp/
│       ├── golang/
│       ├── java/
│       ├── node/
│       ├── objc/
│       ├── php/
│       ├── python/
│       ├── rust/
│       ├── swift/
│       └── typescript/
├── internal/utils/
├── test/example.tars        # Example IDL
└── go.mod
```

## License

BSD 3-Clause License — see the root TarsCpp project for full license text.

## Contributing

Pull requests are welcome. When adding or extending a generator, please update the
table in [What Each Generator Produces](#what-each-generator-produces) so the README
keeps reflecting reality.
