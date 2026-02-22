#include <iostream>
#include <vector>

int main() {
    std::vector<long long> values;
    values.reserve(200001);
    values.push_back(0);
    for (long long i = 0; i < 200000; ++i) {
        values.push_back((i * 9 + 5) % 1000);
    }

    long long checksum = 0;
    for (long long round = 0; round < 40; ++round) {
        long long local = 0;
        for (long long v : values) {
            local += v + (round % 3);
        }
        checksum += local;
    }

    std::cout << checksum << '\n';
    return 0;
}
