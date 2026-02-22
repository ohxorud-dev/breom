#include <iostream>
#include <vector>

int main() {
    std::vector<long long> values;
    values.reserve(3201);
    values.push_back(0);
    for (long long i = 0; i < 3200; ++i) {
        values.push_back((i * 29 + 13) % 20000);
    }

    for (size_t i = 1; i < values.size(); ++i) {
        long long key = values[i];
        size_t j = i;
        while (j > 0 && values[j - 1] > key) {
            values[j] = values[j - 1];
            --j;
        }
        values[j] = key;
    }

    long long checksum = 0;
    for (long long i = 0; i < 128; ++i) {
        checksum += values[i];
    }

    std::cout << checksum << '\n';
    return 0;
}
