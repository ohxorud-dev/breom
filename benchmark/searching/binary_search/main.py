def binary_search(values: list[int], target: int) -> int:
    lo = 0
    hi = len(values) - 1
    while lo <= hi:
        mid = (lo + hi) // 2
        mid_v = values[mid]
        if mid_v == target:
            return mid
        if mid_v < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return -1


def main() -> None:
    values = [0]
    for i in range(40000):
        values.append(i * 2)

    target = 39993 * 2
    checksum = 0
    for i in range(500000):
        idx = binary_search(values, target)
        checksum += (idx % 97) + (i % 3)

    print(checksum)


if __name__ == "__main__":
    main()
