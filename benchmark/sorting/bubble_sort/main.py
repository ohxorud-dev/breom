def main() -> None:
    values = [0]
    for i in range(2200):
        values.append((i * 17 + 23) % 10000)

    n = len(values)
    for i in range(n):
        limit = n - i - 1
        for j in range(limit):
            if values[j] > values[j + 1]:
                values[j], values[j + 1] = values[j + 1], values[j]

    checksum = sum(values[:128])
    print(checksum)


if __name__ == "__main__":
    main()
