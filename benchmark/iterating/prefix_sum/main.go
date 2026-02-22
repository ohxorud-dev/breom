package main

import "fmt"

func main() {
    values := make([]int, 0, 180001)
    values = append(values, 0)
    for i := 0; i < 180000; i++ {
        values = append(values, (i*11+3)%1000)
    }

    checksum := 0
    for round := 0; round < 24; round++ {
        running := 0
        local := 0
        for _, v := range values {
            running += v
            local += (running % 1000) + (round % 7)
        }
        checksum += local
    }

    fmt.Println(checksum)
}
