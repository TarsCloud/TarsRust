package main

import (
	"fmt"
	"net"
	"time"

	"github.com/TarsCloud/TarsGo/tars/protocol/codec"
	"github.com/TarsCloud/TarsGo/tars/protocol/res/requestf"
	"github.com/TarsCloud/TarsGo/tars/util/tools"
)

func main() {
	// Create request packet
	req := &requestf.RequestPacket{
		IVersion:     1,
		CPacketType:  0,
		IMessageType: 0,
		IRequestId:   1,
		SServantName: "Hello.HelloServer.HelloWorldObj",
		SFuncName:    "sayHello",
		ITimeout:     3000,
		Context:      make(map[string]string),
		Status:       make(map[string]string),
	}

	// Encode request body (name and greeting)
	bodyBuf := codec.NewBuffer()
	bodyBuf.WriteString("Rust Client", 1)
	bodyBuf.WriteString("", 2)
	req.SBuffer = tools.ByteToInt8(bodyBuf.ToBytes())

	// Encode request packet
	buf := codec.NewBuffer()
	err := req.WriteTo(buf)
	if err != nil {
		fmt.Println("Error encoding request:", err)
		return
	}

	// Add length prefix
	data := buf.ToBytes()
	length := uint32(len(data) + 4)
	packet := make([]byte, length)
	packet[0] = byte(length >> 24)
	packet[1] = byte(length >> 16)
	packet[2] = byte(length >> 8)
	packet[3] = byte(length)
	copy(packet[4:], data)

	// Print hex dump
	fmt.Printf("Go client request (%d bytes):\n", len(packet))
	for i := 0; i < len(packet); i += 16 {
		fmt.Printf("  %04x: ", i)
		end := i + 16
		if end > len(packet) {
			end = len(packet)
		}
		for j := i; j < end; j++ {
			fmt.Printf("%02x ", packet[j])
		}
		fmt.Print(" |")
		for j := i; j < end; j++ {
			if packet[j] >= 32 && packet[j] <= 126 {
				fmt.Printf("%c", packet[j])
			} else {
				fmt.Print(".")
			}
		}
		fmt.Println("|")
	}

	// Connect and send
	conn, err := net.DialTimeout("tcp", "127.0.0.1:18015", 5*time.Second)
	if err != nil {
		fmt.Println("Connect error:", err)
		return
	}
	defer conn.Close()

	fmt.Println("\nSending to server...")
	_, err = conn.Write(packet)
	if err != nil {
		fmt.Println("Write error:", err)
		return
	}

	// Read response
	resp := make([]byte, 4096)
	conn.SetReadDeadline(time.Now().Add(5 * time.Second))
	n, err := conn.Read(resp)
	if err != nil {
		fmt.Println("Read error:", err)
		return
	}

	fmt.Printf("\nReceived response (%d bytes):\n", n)
	for i := 0; i < n; i += 16 {
		fmt.Printf("  %04x: ", i)
		end := i + 16
		if end > n {
			end = n
		}
		for j := i; j < end; j++ {
			fmt.Printf("%02x ", resp[j])
		}
		fmt.Print(" |")
		for j := i; j < end; j++ {
			if resp[j] >= 32 && resp[j] <= 126 {
				fmt.Printf("%c", resp[j])
			} else {
				fmt.Print(".")
			}
		}
		fmt.Println("|")
	}

	// Parse response
	if n < 4 {
		fmt.Println("Response too short")
		return
	}

	respPacket := &requestf.ResponsePacket{}
	respReader := codec.NewReader(resp[4:n])
	err = respPacket.ReadFrom(respReader)
	if err != nil {
		fmt.Println("Error parsing response:", err)
		return
	}

	fmt.Printf("\nParsed response:\n")
	fmt.Printf("  Version: %d\n", respPacket.IVersion)
	fmt.Printf("  RequestId: %d\n", respPacket.IRequestId)
	fmt.Printf("  Ret: %d\n", respPacket.IRet)
	fmt.Printf("  ResultDesc: %s\n", respPacket.SResultDesc)
	fmt.Printf("  Buffer len: %d\n", len(respPacket.SBuffer))

	if respPacket.IRet == 0 && len(respPacket.SBuffer) > 0 {
		// Parse response body
		bodyReader := codec.NewReader(tools.Int8ToByte(respPacket.SBuffer))
		var ret int32
		var greeting string
		bodyReader.ReadInt32(&ret, 0, true)
		bodyReader.ReadString(&greeting, 2, true)
		fmt.Printf("\nFunction result:\n")
		fmt.Printf("  Return: %d\n", ret)
		fmt.Printf("  Greeting: %s\n", greeting)
	}
}
