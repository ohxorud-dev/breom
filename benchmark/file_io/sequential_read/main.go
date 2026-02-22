package main

import (
	"fmt"
	"os"
)

func main() {
	path := "benchmark/.tmp/io_fixture.txt"
	if len(os.Args) > 1 {
		path = os.Args[1]
	}

	var checksum uint64 = 0
	for i := 0; i < 20; i++ {
		data, err := os.ReadFile(path)
		if err != nil {
			panic(err)
		}
		checksum += uint64(len(data))
	}

	fmt.Println(checksum)
}
