package main

import "fmt"

// HelloWorldImp implements HelloWorld interface
type HelloWorldImp struct {
}

// SayHello returns a greeting message
func (imp *HelloWorldImp) SayHello(name string, greeting *string) (int32, error) {
	*greeting = fmt.Sprintf("Hello, %s! Welcome to Tars.", name)
	return 0, nil
}
