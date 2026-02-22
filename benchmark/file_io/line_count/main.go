package main

import (
	"bufio"
	"fmt"
	"os"
)

func countLines(path string) int {
	f, err := os.Open(path)
	if err != nil {
		panic(err)
	}
	defer f.Close()

	scanner := bufio.NewScanner(f)
	count := 0
	for scanner.Scan() {
		count++
	}
	if err := scanner.Err(); err != nil {
		panic(err)
	}
	return count
}

func main() {
	path := "benchmark/.tmp/io_fixture.txt"
	if len(os.Args) > 1 {
		path = os.Args[1]
	}

	checksum := 0
	for i := 0; i < 15; i++ {
		checksum += countLines(path) + (i % 3)
	}

	fmt.Println(checksum)
}
