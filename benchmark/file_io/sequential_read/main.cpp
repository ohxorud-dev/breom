#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

int main(int argc, char** argv) {
    std::string path = "benchmark/.tmp/io_fixture.txt";
    if (argc > 1) {
        path = argv[1];
    }

    unsigned long long checksum = 0;
    for (int i = 0; i < 20; ++i) {
        std::ifstream input(path, std::ios::binary | std::ios::ate);
        if (!input.is_open()) {
            throw std::runtime_error("failed to open fixture");
        }

        std::streamsize size = input.tellg();
        input.seekg(0, std::ios::beg);

        std::vector<unsigned char> data(size);

        if (input.read(reinterpret_cast<char*>(data.data()), size)) {
            checksum += static_cast<unsigned long long>(data.size());
        } else {
            throw std::runtime_error("failed to read fixture");
        }
    }

    std::cout << checksum << '\n';
    return 0;
}
