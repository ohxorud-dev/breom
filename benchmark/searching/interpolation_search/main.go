package main

import "fmt"

func searchIndex(values []int, target int) int {
    lo := 0
    hi := len(values) - 1
    for lo <= hi {
        mid := (lo + hi) / 2
        v := values[mid]
        if v == target {
            return mid
        }
        if v < target {
            lo = mid + 1
        } else {
            hi = mid - 1
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
