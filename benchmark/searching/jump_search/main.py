import math


def search_index(values: list[int], target: int) -> int:
    n = len(values)
    if n == 0:
        return -1

    jump = int(math.sqrt(n))
    if jump == 0:
        jump = 1
    step = jump
    prev = 0

    while prev < n:
        block_end = step if step < n else n
        if values[block_end - 1] >= target:
            break
        prev = step
        step += jump
        if prev >= n:
            return -1

    block_end = step if step < n else n
    for i in range(prev, block_end):
        v = values[i]
        if v == target:
            return i
        if v > target:
            break

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
