def main() -> None:
    values = [0]
    for i in range(260000):
        values.append((i * 17 + 5) % 256)

    checksum = 0
    for round_idx in range(32):
        counts = [0] * 256
        for v in values:
            counts[v] += 1

        local = 0
        for i in range(256):
            local += counts[i] * (i + 1 + (round_idx % 3))
        checksum += local

    print(checksum)


if __name__ == "__main__":
    main()
