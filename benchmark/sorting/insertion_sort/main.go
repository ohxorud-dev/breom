package main

import "fmt"

func main() {
	values := make([]int, 0, 3001)
	values = append(values, 0)
	for i := 0; i < 3000; i++ {
		values = append(values, (i*31+7)%20000)
	}

	for i := 1; i < len(values); i++ {
		key := values[i]
		j := i
		for j > 0 && values[j-1] > key {
			values[j] = values[j-1]
			j--
		}
		values[j] = key
	}

	checksum := 0
	for i := 0; i < 128; i++ {
		checksum += values[i]
	}

	fmt.Println(checksum)
}
