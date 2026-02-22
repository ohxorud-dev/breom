import sys


def line_length_sum(path: str) -> int:
    total = 0
    with open(path, "r", encoding="utf-8", newline="") as f:
        for line in f:
            total += len(line.rstrip("\r\n"))
    return total


def main() -> None:
    path = sys.argv[1] if len(sys.argv) > 1 else "benchmark/.tmp/io_fixture.txt"

    checksum = 0
    for i in range(12):
        checksum += line_length_sum(path) + (i % 4)

    print(checksum)


if __name__ == "__main__":
    main()
