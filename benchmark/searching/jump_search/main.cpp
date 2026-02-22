#include <cmath>
#include <iostream>
#include <vector>

long long search_index(const std::vector<long long>& values, long long target) {
    const size_t n = values.size();
    if (n == 0) {
        return -1;
    }

    size_t jump = static_cast<size_t>(std::sqrt(static_cast<double>(n)));
    if (jump == 0) {
        jump = 1;
    }
    size_t step = jump;
    size_t prev = 0;

    while (prev < n) {
        const size_t block_end = step < n ? step : n;
        if (values[block_end - 1] >= target) {
            break;
        }
        prev = step;
        step += jump;
        if (prev >= n) {
            return -1;
        }
    }

    const size_t block_end = step < n ? step : n;
    for (size_t i = prev; i < block_end; ++i) {
        const long long v = values[i];
        if (v == target) {
            return static_cast<long long>(i);
        }
        if (v > target) {
            break;
        }
    }

    return -1;
}

int main() {
    std::vector<long long> values;
    values.reserve(50001);
    values.push_back(0);
    for (long long i = 0; i < 50000; ++i) {
        values.push_back(i * 2);
    }

    long long target = 49991LL * 2;
    long long checksum = 0;
    for (long long i = 0; i < 420000; ++i) {
        long long idx = search_index(values, target);
        checksum += (idx % 97) + (i % 5);
    }

    std::cout << checksum << '\n';
    return 0;
}
