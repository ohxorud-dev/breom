def main() -> None:
    values = [0]
    for i in range(3200):
        values.append((i * 29 + 13) % 20000)

    for i in range(len(values)):
        min_index = i
        for j in range(i + 1, len(values)):
            if values[j] < values[min_index]:
                min_index = j
        if min_index != i:
            values[i], values[min_index] = values[min_index], values[i]

    checksum = sum(values[:128])
    print(checksum)


if __name__ == "__main__":
    main()
