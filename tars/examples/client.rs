//! Rust client to test calling Go Tars server

use std::collections::HashMap;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Constants from Tars protocol
const TARS_VERSION: i16 = 1;
const TARS_NORMAL: i8 = 0;

// TLV types
const TYPE_INT8: u8 = 0;
const TYPE_INT16: u8 = 1;
const TYPE_INT32: u8 = 2;
const TYPE_STRING1: u8 = 6;
const TYPE_STRING4: u8 = 7;
const TYPE_MAP: u8 = 8;
const TYPE_STRUCT_BEGIN: u8 = 10;
const TYPE_STRUCT_END: u8 = 11;
const TYPE_ZERO_TAG: u8 = 12;

/// TLV buffer writer
struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn write_head(&mut self, ty: u8, tag: u8) {
        // IMPORTANT: Go uses (tag << 4) | ty, not (ty << 4) | tag
        if tag < 15 {
            self.data.push((tag << 4) | ty);
        } else {
            self.data.push((15 << 4) | ty);
            self.data.push(tag);
        }
    }

    fn write_int8(&mut self, val: i8, tag: u8) {
        if val == 0 {
            self.write_head(TYPE_ZERO_TAG, tag);
        } else {
            self.write_head(TYPE_INT8, tag);
            self.data.push(val as u8);
        }
    }

    fn write_int16(&mut self, val: i16, tag: u8) {
        if val >= i8::MIN as i16 && val <= i8::MAX as i16 {
            self.write_int8(val as i8, tag);
        } else {
            self.write_head(TYPE_INT16, tag);
            self.data.extend_from_slice(&val.to_be_bytes());
        }
    }

    fn write_int32(&mut self, val: i32, tag: u8) {
        if val >= i16::MIN as i32 && val <= i16::MAX as i32 {
            self.write_int16(val as i16, tag);
        } else {
            self.write_head(TYPE_INT32, tag);
            self.data.extend_from_slice(&val.to_be_bytes());
        }
    }

    fn write_string(&mut self, val: &str, tag: u8) {
        let bytes = val.as_bytes();
        if bytes.len() < 256 {
            self.write_head(TYPE_STRING1, tag);
            self.data.push(bytes.len() as u8);
        } else {
            self.write_head(TYPE_STRING4, tag);
            self.data.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        }
        self.data.extend_from_slice(bytes);
    }

    fn write_map_begin(&mut self, size: i32, tag: u8) {
        self.write_head(TYPE_MAP, tag);
        self.write_int32(size, 0);
    }

    fn write_struct_begin(&mut self, tag: u8) {
        self.write_head(TYPE_STRUCT_BEGIN, tag);
    }

    fn write_struct_end(&mut self) {
        self.write_head(TYPE_STRUCT_END, 0);
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn to_bytes_with_len(&self) -> Vec<u8> {
        let len = self.data.len() as u32 + 4;
        let mut result = Vec::with_capacity(len as usize);
        result.extend_from_slice(&len.to_be_bytes());
        result.extend_from_slice(&self.data);
        result
    }
}

/// Encode RequestPacket
fn encode_request(
    version: i16,
    packet_type: i8,
    request_id: i32,
    servant_name: &str,
    func_name: &str,
    buffer: &[u8],
    timeout: i32,
    status: &HashMap<String, String>,
    context: &HashMap<String, String>,
) -> Vec<u8> {
    let mut buf = Buffer::new();

    // iVersion (tag 1)
    buf.write_int16(version, 1);
    // cPacketType (tag 2)
    buf.write_int8(packet_type, 2);
    // iMessageType (tag 3) - 0
    buf.write_int32(0, 3);
    // iRequestId (tag 4)
    buf.write_int32(request_id, 4);
    // sServantName (tag 5)
    buf.write_string(servant_name, 5);
    // sFuncName (tag 6)
    buf.write_string(func_name, 6);
    // sBuffer (tag 7)
    buf.write_head(13, 7); // SimpleList
    buf.write_head(TYPE_INT8, 0);
    buf.write_int32(buffer.len() as i32, 0);
    buf.data.extend_from_slice(buffer);
    // iTimeout (tag 8)
    buf.write_int32(timeout, 8);
    // context (tag 9)
    buf.write_map_begin(context.len() as i32, 9);
    for (k, v) in context {
        buf.write_string(k, 0);
        buf.write_string(v, 1);
    }
    // status (tag 10)
    buf.write_map_begin(status.len() as i32, 10);
    for (k, v) in status {
        buf.write_string(k, 0);
        buf.write_string(v, 1);
    }

    buf.to_bytes_with_len()
}

/// TLV reader
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn peek_head(&self) -> Option<(u8, u8)> {
        if self.pos >= self.data.len() {
            return None;
        }
        let b = self.data[self.pos];
        // IMPORTANT: Go uses (tag << 4) | ty
        let tag = (b & 0xF0) >> 4;
        let ty = b & 0x0F;
        if tag == 15 {
            if self.pos + 1 >= self.data.len() {
                return None;
            }
            Some((ty, self.data[self.pos + 1]))
        } else {
            Some((ty, tag))
        }
    }

    fn read_head(&mut self) -> Option<(u8, u8)> {
        let (ty, tag) = self.peek_head()?;
        // Check if high nibble (tag position) is 15
        let first_byte_tag = (self.data[self.pos] & 0xF0) >> 4;
        self.pos += if first_byte_tag == 15 { 2 } else { 1 };
        Some((ty, tag))
    }

    fn skip_to_tag(&mut self, target: u8) -> bool {
        loop {
            if let Some((ty, tag)) = self.peek_head() {
                if tag == target {
                    return true;
                }
                if tag > target {
                    return false;
                }
                // Skip this field
                self.read_head();
                self.skip_field(ty);
            } else {
                return false;
            }
        }
    }

    fn skip_field(&mut self, ty: u8) {
        match ty {
            0 => self.pos += 1, // int8
            1 => self.pos += 2, // int16
            2 => self.pos += 4, // int32
            3 => self.pos += 8, // int64
            4 => self.pos += 4, // float
            5 => self.pos += 8, // double
            6 => { // string1
                if self.pos < self.data.len() {
                    let len = self.data[self.pos] as usize;
                    self.pos += 1 + len;
                }
            }
            7 => { // string4
                if self.pos + 4 <= self.data.len() {
                    let len = u32::from_be_bytes([
                        self.data[self.pos],
                        self.data[self.pos + 1],
                        self.data[self.pos + 2],
                        self.data[self.pos + 3],
                    ]) as usize;
                    self.pos += 4 + len;
                }
            }
            8 => { // map
                let size = self.read_int32_raw();
                for _ in 0..size {
                    if let Some((kty, _)) = self.read_head() { self.skip_field(kty); }
                    if let Some((vty, _)) = self.read_head() { self.skip_field(vty); }
                }
            }
            9 => { // list
                let size = self.read_int32_raw();
                for _ in 0..size {
                    if let Some((ety, _)) = self.read_head() { self.skip_field(ety); }
                }
            }
            10 => { // struct begin
                loop {
                    if let Some((sty, _)) = self.read_head() {
                        if sty == 11 { break; }
                        self.skip_field(sty);
                    } else {
                        break;
                    }
                }
            }
            12 => {} // zero tag - no data
            13 => { // simple list (bytes)
                self.read_head(); // skip inner head
                let size = self.read_int32_raw();
                self.pos += size as usize;
            }
            _ => {}
        }
    }

    fn read_int32_raw(&mut self) -> i32 {
        if let Some((ty, _)) = self.read_head() {
            match ty {
                0 => {
                    let v = self.data[self.pos] as i8;
                    self.pos += 1;
                    v as i32
                }
                1 => {
                    let v = i16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
                    self.pos += 2;
                    v as i32
                }
                2 => {
                    let v = i32::from_be_bytes([
                        self.data[self.pos],
                        self.data[self.pos + 1],
                        self.data[self.pos + 2],
                        self.data[self.pos + 3],
                    ]);
                    self.pos += 4;
                    v
                }
                12 => 0, // zero
                _ => 0,
            }
        } else {
            0
        }
    }

    fn read_int32(&mut self, tag: u8) -> Option<i32> {
        if self.skip_to_tag(tag) {
            Some(self.read_int32_raw())
        } else {
            None
        }
    }

    fn read_string(&mut self, tag: u8) -> Option<String> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        if let Some((ty, _)) = self.read_head() {
            let len = match ty {
                6 => {
                    let len = self.data[self.pos] as usize;
                    self.pos += 1;
                    len
                }
                7 => {
                    let len = u32::from_be_bytes([
                        self.data[self.pos],
                        self.data[self.pos + 1],
                        self.data[self.pos + 2],
                        self.data[self.pos + 3],
                    ]) as usize;
                    self.pos += 4;
                    len
                }
                _ => return None,
            };
            let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len]).to_string();
            self.pos += len;
            Some(s)
        } else {
            None
        }
    }

    fn read_bytes(&mut self, tag: u8) -> Option<Vec<u8>> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        if let Some((ty, _)) = self.read_head() {
            if ty == 13 { // SimpleList
                self.read_head(); // Skip inner type head
                let size = self.read_int32_raw() as usize;
                let bytes = self.data[self.pos..self.pos + size].to_vec();
                self.pos += size;
                return Some(bytes);
            }
        }
        None
    }
}

/// Parse response packet
fn parse_response(data: &[u8]) -> Result<(i32, i32, String, Vec<u8>), String> {
    // Skip 4-byte length header
    if data.len() < 4 {
        return Err("Response too short".into());
    }

    let mut reader = Reader::new(&data[4..]);

    // iVersion (tag 1)
    let _version = reader.read_int32(1).unwrap_or(1);
    // cPacketType (tag 2) - skip
    let _ = reader.read_int32(2);
    // iRequestId (tag 3)
    let request_id = reader.read_int32(3).unwrap_or(0);
    // iMessageType (tag 4) - skip
    let _ = reader.read_int32(4);
    // iRet (tag 5)
    let ret = reader.read_int32(5).unwrap_or(0);
    // sBuffer (tag 6)
    let buffer = reader.read_bytes(6).unwrap_or_default();
    // status (tag 7) - skip
    // sResultDesc (tag 8)
    let result_desc = reader.read_string(8).unwrap_or_default();

    Ok((request_id, ret, result_desc, buffer))
}

/// Encode sayHello request body
fn encode_say_hello_request(name: &str) -> Vec<u8> {
    let mut buf = Buffer::new();
    buf.write_string(name, 1);       // name at tag 1
    buf.write_string("", 2);         // greeting (out param) at tag 2, empty initially
    buf.to_bytes()
}

/// Parse sayHello response body
fn parse_say_hello_response(buffer: &[u8]) -> Result<(i32, String), String> {
    let mut reader = Reader::new(buffer);

    // Return value at tag 0
    let ret = reader.read_int32(0).unwrap_or(-1);
    // greeting at tag 2
    let greeting = reader.read_string(2).unwrap_or_default();

    Ok((ret, greeting))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rust Tars Client Test ===\n");

    // Connect to the Go Tars server
    let addr = "127.0.0.1:18015";
    println!("Connecting to {}...", addr);

    let mut stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    println!("Connected!\n");

    // Prepare the request
    let servant_name = "Hello.HelloServer.HelloWorldObj";
    let func_name = "sayHello";
    let name = "Rust Client";

    println!("Calling {}.{}(\"{}\")...\n", servant_name, func_name, name);

    // Encode request body
    let body = encode_say_hello_request(name);

    // Encode full request packet
    let request = encode_request(
        TARS_VERSION,
        TARS_NORMAL,
        1,  // request id
        servant_name,
        func_name,
        &body,
        3000, // timeout ms
        &HashMap::new(),
        &HashMap::new(),
    );

    println!("Sending {} bytes request...", request.len());

    // Debug: print raw request bytes
    println!("Raw request (hex):");
    for (i, chunk) in request.chunks(16).enumerate() {
        print!("  {:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        // Show ASCII
        print!(" |");
        for b in chunk {
            if *b >= 32 && *b <= 126 {
                print!("{}", *b as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
    println!();

    stream.write_all(&request).await?;

    // Read response
    let mut response = vec![0u8; 4096];
    let timeout = Duration::from_secs(5);
    let n = tokio::time::timeout(timeout, stream.read(&mut response)).await??;
    response.truncate(n);

    println!("Received {} bytes response\n", n);

    // Debug: print raw response bytes
    println!("Raw response (hex):");
    for (i, chunk) in response.chunks(16).enumerate() {
        print!("  {:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        println!();
    }
    println!();

    // Parse response
    let (req_id, ret_code, result_desc, buffer) = parse_response(&response)?;

    println!("Response packet:");
    println!("  Request ID: {}", req_id);
    println!("  Return Code: {}", ret_code);
    println!("  Result Desc: {}", result_desc);
    println!("  Buffer Length: {}", buffer.len());

    if ret_code == 0 {
        let (func_ret, greeting) = parse_say_hello_response(&buffer)?;
        println!("\nFunction result:");
        println!("  Return: {}", func_ret);
        println!("  Greeting: \"{}\"", greeting);

        if func_ret == 0 && greeting.contains("Hello") {
            println!("\n=== TEST PASSED ===");
        } else {
            println!("\n=== TEST FAILED: Unexpected response ===");
        }
    } else {
        println!("\n=== TEST FAILED: Server returned error {} ===", ret_code);
    }

    Ok(())
}
