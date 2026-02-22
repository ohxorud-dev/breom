#include <iostream>
#include <vector>

static long long binary_search_index(const std::vector<long long>& values, long long target) {
    long long lo = 0;
    long long hi = static_cast<long long>(values.size()) - 1;

    while (lo <= hi) {
        const long long mid = (lo + hi) / 2;
        const long long mid_v = values[static_cast<size_t>(mid)];
        if (mid_v == target) {
            return mid;
        }
        if (mid_v < target) {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }

    return -1;
}

int main() {
    std::vector<long long> values;
    values.reserve(40001);
    values.push_back(0);
    for (long long i = 0; i < 40000; ++i) {
        values.push_back(i * 2);
    }

    const long long target = 39993 * 2;
    long long checksum = 0;
    for (long long i = 0; i < 500000; ++i) {
        const long long idx = binary_search_index(values, target);
        checksum += (idx % 97) + (i % 3);
    }

    std::cout << checksum << '\n';
    return 0;
}
