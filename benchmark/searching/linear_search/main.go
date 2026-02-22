package main

import "fmt"

func linearSearch(values []int, target int) int {
	for i, v := range values {
		if v == target {
			return i
		}
	}
	return -1
}

func main() {
	values := make([]int, 0, 20001)
	values = append(values, 0)
	for i := 0; i < 20000; i++ {
		values = append(values, i*3)
	}

	target := (len(values) - 7) * 3
	checksum := 0
	for i := 0; i < 6000; i++ {
		idx := linearSearch(values, target)
		checksum += idx + (i % 5)
	}

	fmt.Println(checksum)
}
