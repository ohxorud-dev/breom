import sys


def main() -> None:
    path = sys.argv[1] if len(sys.argv) > 1 else "benchmark/.tmp/io_fixture.txt"

    checksum = 0
    for _ in range(20):
        with open(path, "rb") as f:
            data = f.read()
        checksum += len(data)

    print(checksum)


if __name__ == "__main__":
    main()
