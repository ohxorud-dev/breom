package main

import "fmt"

func main() {
	values := make([]int, 0, 3201)
	values = append(values, 0)
	for i := 0; i < 3200; i++ {
		values = append(values, (i*29+13)%20000)
	}

	n := len(values)
	temp := make([]int, n)
	for width := 1; width < n; width *= 2 {
		for left := 0; left < n; left += 2 * width {
			mid := left + width
			if mid > n {
				mid = n
			}

			right := left + 2*width
			if right > n {
				right = n
			}

			i, j, k := left, mid, left
			for i < mid && j < right {
				if values[i] <= values[j] {
					temp[k] = values[i]
					i++
				} else {
					temp[k] = values[j]
					j++
				}
				k++
			}

			for i < mid {
				temp[k] = values[i]
				i++
				k++
			}

			for j < right {
				temp[k] = values[j]
				j++
				k++
			}
		}

		copy(values, temp)
	}

	checksum := 0
	for i := 0; i < 128; i++ {
		checksum += values[i]
	}

	fmt.Println(checksum)
}
