def main() -> None:
    checksum = 0

    for outer in range(300):
        row = 0
        for inner in range(4000):
            row += ((outer + 1) * (inner + 3)) % 97
        checksum += row

    print(checksum)


if __name__ == "__main__":
    main()
