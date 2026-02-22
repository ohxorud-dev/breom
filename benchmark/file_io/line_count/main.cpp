#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>

static long long count_lines(const std::string& path) {
    std::ifstream input(path);
    if (!input.is_open()) {
        throw std::runtime_error("failed to open fixture");
    }

    long long count = 0;
    std::string line;
    while (std::getline(input, line)) {
        ++count;
    }
    return count;
}

int main(int argc, char** argv) {
    std::string path = "benchmark/.tmp/io_fixture.txt";
    if (argc > 1) {
        path = argv[1];
    }

    long long checksum = 0;
    for (long long i = 0; i < 15; ++i) {
        checksum += count_lines(path) + (i % 3);
    }

    std::cout << checksum << '\n';
    return 0;
}
