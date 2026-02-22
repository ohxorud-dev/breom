def search_index(values: list[int], target: int) -> int:
    lo = 0
    hi = len(values)
    while lo < hi:
        mid = (lo + hi) // 2
        if values[mid] < target:
            lo = mid + 1
        else:
            hi = mid
    if lo < len(values) and values[lo] == target:
        return lo
    return -1


def main() -> None:
    values = [0]
    for i in range(50000):
        values.append(i * 2)

    target = 49991 * 2
    checksum = 0
    for i in range(420000):
        idx = search_index(values, target)
        checksum += (idx % 97) + (i % 5)

    print(checksum)


if __name__ == "__main__":
    main()
