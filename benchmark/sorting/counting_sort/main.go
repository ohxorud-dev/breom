package main

import "fmt"

func main() {
    values := make([]int, 0, 3201)
    values = append(values, 0)
    for i := 0; i < 3200; i++ {
        values = append(values, (i*29+13)%20000)
    }

    for i := 1; i < len(values); i++ {
        key := values[i]
        j := i
        for j > 0 && values[j-1] > key {
            values[j] = values[j-1]
            j--
        }
        values[j] = key
    }

    checksum := 0
    for i := 0; i < 128; i++ {
        checksum += values[i]
    }

    fmt.Println(checksum)
}
