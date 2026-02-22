#include <iostream>
#include <vector>

static long long linear_search_index(const std::vector<long long>& values, long long target) {
    for (size_t i = 0; i < values.size(); ++i) {
        if (values[i] == target) {
            return static_cast<long long>(i);
        }
    }
    return -1;
}

int main() {
    std::vector<long long> values;
    values.reserve(20001);
    values.push_back(0);
    for (long long i = 0; i < 20000; ++i) {
        values.push_back(i * 3);
    }

    const long long target = (static_cast<long long>(values.size()) - 7) * 3;
    long long checksum = 0;
    for (long long i = 0; i < 6000; ++i) {
        const long long idx = linear_search_index(values, target);
        checksum += idx + (i % 5);
    }

    std::cout << checksum << '\n';
    return 0;
}
