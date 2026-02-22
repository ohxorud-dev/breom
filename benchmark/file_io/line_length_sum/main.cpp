#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>

unsigned long long line_length_sum(const std::string& path) {
    std::ifstream input(path);
    if (!input.is_open()) {
        throw std::runtime_error("failed to open fixture");
    }

    unsigned long long total = 0;
    std::string line;
    while (std::getline(input, line)) {
        if (!line.empty() && line.back() == '\r') {
            line.pop_back();
        }
        total += static_cast<unsigned long long>(line.size());
    }
    return total;
}

int main(int argc, char** argv) {
    std::string path = "benchmark/.tmp/io_fixture.txt";
    if (argc > 1) {
        path = argv[1];
    }

    unsigned long long checksum = 0;
    for (unsigned long long i = 0; i < 12; ++i) {
        checksum += line_length_sum(path) + (i % 4);
    }

    std::cout << checksum << '\n';
    return 0;
}
