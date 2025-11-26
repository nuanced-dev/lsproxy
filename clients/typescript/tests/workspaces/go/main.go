package main

import "fmt"

func main() {
	total := Add(2, 3)
	g := Greeter{"Nuanced"}
	series := make([]string, 0, 20)
	for i := 1; i <= 20; i++ {
		series = append(series, FizzBuzz(i))
	}
	fmt.Println(total, g.Greet(), series)
}
