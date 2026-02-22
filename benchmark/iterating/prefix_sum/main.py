def main() -> None:
    values = [0]
    for i in range(180000):
        values.append((i * 11 + 3) % 1000)

    checksum = 0
    for round_idx in range(24):
        running = 0
        local = 0
        for v in values:
            running += v
            local += (running % 1000) + (round_idx % 7)
        checksum += local

    print(checksum)


if __name__ == "__main__":
    main()
