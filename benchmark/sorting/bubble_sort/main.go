package main

import "fmt"

func main() {
	values := make([]int, 0, 2201)
	values = append(values, 0)
	for i := 0; i < 2200; i++ {
		values = append(values, (i*17+23)%10000)
	}

	n := len(values)
	for i := 0; i < n; i++ {
		limit := n - i - 1
		for j := 0; j < limit; j++ {
			if values[j] > values[j+1] {
				values[j], values[j+1] = values[j+1], values[j]
			}
		}
	}

	checksum := 0
	for i := 0; i < 128; i++ {
		checksum += values[i]
	}

	fmt.Println(checksum)
}
