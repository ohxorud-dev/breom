def linear_search(values: list[int], target: int) -> int:
    for i, value in enumerate(values):
        if value == target:
            return i
    return -1


def main() -> None:
    values = [0]
    for i in range(20000):
        values.append(i * 3)

    target = (len(values) - 7) * 3
    checksum = 0
    for i in range(6000):
        idx = linear_search(values, target)
        checksum += idx + (i % 5)

    print(checksum)


if __name__ == "__main__":
    main()
