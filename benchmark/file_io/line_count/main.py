import sys


def count_lines(path: str) -> int:
    with open(path, "r", encoding="utf-8") as f:
        return sum(1 for _ in f)


def main() -> None:
    path = sys.argv[1] if len(sys.argv) > 1 else "benchmark/.tmp/io_fixture.txt"

    checksum = 0
    for i in range(15):
        checksum += count_lines(path) + (i % 3)

    print(checksum)


if __name__ == "__main__":
    main()
