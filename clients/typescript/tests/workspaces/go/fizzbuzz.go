package main

func FizzBuzz(n int) string {
	switch {
	case n%15 == 0:
		return "fizzbuzz"
	case n%3 == 0:
		return "fizz"
	case n%5 == 0:
		return "buzz"
	default:
		return fmtInt(n)
	}
}
