package main

import (
	"fmt"
	"math"
)

func searchIndex(values []int, target int) int {
	n := len(values)
	if n == 0 {
		return -1
	}

	jump := int(math.Sqrt(float64(n)))
	if jump == 0 {
		jump = 1
	}
	step := jump
	prev := 0

	for prev < n {
		blockEnd := step
		if blockEnd > n {
			blockEnd = n
		}
		if values[blockEnd-1] >= target {
			break
		}
		prev = step
		step += jump
		if prev >= n {
			return -1
		}
	}

	blockEnd := step
	if blockEnd > n {
		blockEnd = n
	}
	for i := prev; i < blockEnd; i++ {
		v := values[i]
		if v == target {
			return i
		}
		if v > target {
			break
		}
	}

	return -1
}

func main() {
	values := make([]int, 0, 50001)
	values = append(values, 0)
	for i := 0; i < 50000; i++ {
		values = append(values, i*2)
	}

	target := 49991 * 2
	checksum := 0
	for i := 0; i < 420000; i++ {
		idx := searchIndex(values, target)
		checksum += (idx % 97) + (i % 5)
	}

	fmt.Println(checksum)
}
