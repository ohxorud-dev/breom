package main

import (
	"bufio"
	"fmt"
	"os"
)

func lineLengthSum(path string) uint64 {
	file, err := os.Open(path)
	if err != nil {
		panic(err)
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	buf := make([]byte, 0, 1024)
	scanner.Buffer(buf, 1024*1024)

	var total uint64 = 0
	for scanner.Scan() {
		total += uint64(len(scanner.Bytes()))
	}
	if err := scanner.Err(); err != nil {
		panic(err)
	}
	return total
}

func main() {
	path := "benchmark/.tmp/io_fixture.txt"
	if len(os.Args) > 1 {
		path = os.Args[1]
	}

	var checksum uint64 = 0
	for i := 0; i < 12; i++ {
		checksum += lineLengthSum(path) + uint64(i%4)
	}

	fmt.Println(checksum)
}
