#include <array>
#include <iostream>
#include <vector>

int main() {
    std::vector<int> values;
    values.reserve(260001);
    values.push_back(0);
    for (int i = 0; i < 260000; ++i) {
        values.push_back((i * 17 + 5) % 256);
    }

    unsigned long long checksum = 0;
    for (int round = 0; round < 32; ++round) {
        std::array<unsigned long long, 256> counts{};
        for (int v : values) {
            counts[static_cast<size_t>(v)]++;
        }

        unsigned long long local = 0;
        for (int i = 0; i < 256; ++i) {
            local += counts[static_cast<size_t>(i)] * static_cast<unsigned long long>(i + 1 + (round % 3));
        }
        checksum += local;
    }

    std::cout << checksum << '\n';
    return 0;
}
