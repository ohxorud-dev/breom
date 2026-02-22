#include <iostream>
#include <vector>

int main() {
    std::vector<long long> values;
    values.reserve(180001);
    values.push_back(0);
    for (long long i = 0; i < 180000; ++i) {
        values.push_back((i * 11 + 3) % 1000);
    }

    long long checksum = 0;
    for (long long round = 0; round < 24; ++round) {
        long long running = 0;
        long long local = 0;
        for (long long v : values) {
            running += v;
            local += (running % 1000) + (round % 7);
        }
        checksum += local;
    }

    std::cout << checksum << '\n';
    return 0;
}
