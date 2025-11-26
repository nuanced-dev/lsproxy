package main

import "fmt"

func Add(a, b int) int { return a + b }

type Greeter struct{ Name string }

func (g Greeter) Greet() string { return fmt.Sprintf("Hello, %s!", g.Name) }

func fmtInt(n int) string { return fmt.Sprintf("%d", n) }
