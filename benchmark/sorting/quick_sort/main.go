package main

import "fmt"

func partition(values []int, low int, high int) int {
	pivot := values[high]
	i := low
	for j := low; j < high; j++ {
		if values[j] <= pivot {
			values[i], values[j] = values[j], values[i]
			i++
		}
	}
	values[i], values[high] = values[high], values[i]
	return i
}

func quickSort(values []int, low int, high int) {
	if low >= high {
		return
	}

	p := partition(values, low, high)
	if p > 0 {
		quickSort(values, low, p-1)
	}
	quickSort(values, p+1, high)
}

func main() {
	values := make([]int, 0, 3201)
	values = append(values, 0)
	for i := 0; i < 3200; i++ {
		values = append(values, (i*29+13)%20000)
	}

	if len(values) > 0 {
		quickSort(values, 0, len(values)-1)
	}

	checksum := 0
	for i := 0; i < 128; i++ {
		checksum += values[i]
	}

	fmt.Println(checksum)
}
