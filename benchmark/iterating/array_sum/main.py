def main() -> None:
    values = [0]
    for i in range(200000):
        values.append((i * 9 + 5) % 1000)

    checksum = 0
    for round_idx in range(40):
        local = 0
        for v in values:
            local += v + (round_idx % 3)
        checksum += local

    print(checksum)


if __name__ == "__main__":
    main()
