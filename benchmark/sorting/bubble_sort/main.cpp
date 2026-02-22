#include <iostream>
#include <vector>

int main() {
    std::vector<long long> values;
    values.reserve(2201);
    values.push_back(0);
    for (long long i = 0; i < 2200; ++i) {
        values.push_back((i * 17 + 23) % 10000);
    }

    const long long n = static_cast<long long>(values.size());
    for (long long i = 0; i < n; ++i) {
        const long long limit = n - i - 1;
        for (long long j = 0; j < limit; ++j) {
            if (values[j] > values[j + 1]) {
                std::swap(values[j], values[j + 1]);
            }
        }
    }

    long long checksum = 0;
    for (long long i = 0; i < 128; ++i) {
        checksum += values[i];
    }

    std::cout << checksum << '\n';
    return 0;
}
