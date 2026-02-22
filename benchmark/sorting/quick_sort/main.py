def partition(values: list[int], low: int, high: int) -> int:
    pivot = values[high]
    i = low
    for j in range(low, high):
        if values[j] <= pivot:
            values[i], values[j] = values[j], values[i]
            i += 1
    values[i], values[high] = values[high], values[i]
    return i


def quick_sort(values: list[int], low: int, high: int) -> None:
    if low >= high:
        return

    pivot_index = partition(values, low, high)
    if pivot_index > 0:
        quick_sort(values, low, pivot_index - 1)
    quick_sort(values, pivot_index + 1, high)


def main() -> None:
    values = [0]
    for i in range(3200):
        values.append((i * 29 + 13) % 20000)

    if values:
        quick_sort(values, 0, len(values) - 1)

    checksum = sum(values[:128])
    print(checksum)


if __name__ == "__main__":
    main()
