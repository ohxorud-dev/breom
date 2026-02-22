package main

import "fmt"

func main() {
	values := make([]int, 0, 3201)
	values = append(values, 0)
	for i := 0; i < 3200; i++ {
		values = append(values, (i*29+13)%20000)
	}

	for i := 0; i < len(values); i++ {
		minIndex := i
		for j := i + 1; j < len(values); j++ {
			if values[j] < values[minIndex] {
				minIndex = j
			}
		}

		if minIndex != i {
			values[i], values[minIndex] = values[minIndex], values[i]
		}
	}

	checksum := 0
	for i := 0; i < 128; i++ {
		checksum += values[i]
	}

	fmt.Println(checksum)
}
