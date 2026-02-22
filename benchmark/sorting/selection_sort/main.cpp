#include <iostream>
#include <vector>

int main() {
    std::vector<long long> values;
    values.reserve(3201);
    values.push_back(0);
    for (long long i = 0; i < 3200; ++i) {
        values.push_back((i * 29 + 13) % 20000);
    }

    for (size_t i = 0; i < values.size(); ++i) {
        size_t min_index = i;
        for (size_t j = i + 1; j < values.size(); ++j) {
            if (values[j] < values[min_index]) {
                min_index = j;
            }
        }

        if (min_index != i) {
            std::swap(values[i], values[min_index]);
        }
    }

    long long checksum = 0;
    for (long long i = 0; i < 128; ++i) {
        checksum += values[i];
    }

    std::cout << checksum << '\n';
    return 0;
}
