def main() -> None:
    values = [0]
    for i in range(3200):
        values.append((i * 29 + 13) % 20000)

    n = len(values)
    temp = [0] * n
    width = 1
    while width < n:
        left = 0
        while left < n:
            mid = min(left + width, n)
            right = min(left + 2 * width, n)

            i, j, k = left, mid, left
            while i < mid and j < right:
                if values[i] <= values[j]:
                    temp[k] = values[i]
                    i += 1
                else:
                    temp[k] = values[j]
                    j += 1
                k += 1

            while i < mid:
                temp[k] = values[i]
                i += 1
                k += 1

            while j < right:
                temp[k] = values[j]
                j += 1
                k += 1

            left += 2 * width

        values[:] = temp
        width *= 2

    checksum = sum(values[:128])
    print(checksum)


if __name__ == "__main__":
    main()
