package main

import (
	"fmt"

	"github.com/TarsCloud/TarsGo/tars"

	"HelloServer/tars-protocol/Hello"
)

func main() {
	cfg := tars.GetServerConfig()
	fmt.Printf("Starting server: %s.%s\n", cfg.App, cfg.Server)

	imp := new(HelloWorldImp)
	app := new(Hello.HelloWorld)
	app.AddServant(imp, cfg.App+"."+cfg.Server+".HelloWorldObj")

	fmt.Println("Server is running...")
	tars.Run()
}
