package main

import "fmt"

func main() {
	values := make([]int, 0, 200001)
	values = append(values, 0)
	for i := 0; i < 200000; i++ {
		values = append(values, (i*9+5)%1000)
	}

	checksum := 0
	for round := 0; round < 40; round++ {
		local := 0
		for _, v := range values {
			local += v + (round % 3)
		}
		checksum += local
	}

	fmt.Println(checksum)
}
