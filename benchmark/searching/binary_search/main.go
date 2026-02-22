package main

import "fmt"

func binarySearch(values []int, target int) int {
	lo := 0
	hi := len(values) - 1

	for lo <= hi {
		mid := (lo + hi) / 2
		midV := values[mid]
		if midV == target {
			return mid
		}
		if midV < target {
			lo = mid + 1
		} else {
			hi = mid - 1
		}
	}

	return -1
}

func main() {
	values := make([]int, 0, 40001)
	values = append(values, 0)
	for i := 0; i < 40000; i++ {
		values = append(values, i*2)
	}

	target := 39993 * 2
	checksum := 0
	for i := 0; i < 500000; i++ {
		idx := binarySearch(values, target)
		checksum += (idx % 97) + (i % 3)
	}

	fmt.Println(checksum)
}
