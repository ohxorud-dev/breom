package main

import "fmt"

func main() {
    values := make([]int, 0, 260001)
    values = append(values, 0)
    for i := 0; i < 260000; i++ {
        values = append(values, (i*17+5)%256)
    }

    var checksum uint64 = 0
    for round := 0; round < 32; round++ {
        counts := [256]uint64{}
        for _, v := range values {
            counts[v]++
        }

        var local uint64 = 0
        for i := 0; i < 256; i++ {
            local += counts[i] * uint64(i+1+(round%3))
        }
        checksum += local
    }

    fmt.Println(checksum)
}
