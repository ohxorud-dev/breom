package main

import "fmt"

func main() {
	checksum := 0

	for outer := 0; outer < 300; outer++ {
		row := 0
		for inner := 0; inner < 4000; inner++ {
			row += ((outer + 1) * (inner + 3)) % 97
		}
		checksum += row
	}

	fmt.Println(checksum)
}
