def main() -> None:
    values = [0]
    for i in range(3200):
        values.append((i * 29 + 13) % 20000)

    for i in range(1, len(values)):
        key = values[i]
        j = i
        while j > 0 and values[j - 1] > key:
            values[j] = values[j - 1]
            j -= 1
        values[j] = key

    checksum = sum(values[:128])
    print(checksum)


if __name__ == "__main__":
    main()
