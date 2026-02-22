#include <iostream>
#include <vector>

long long search_index(const std::vector<long long>& values, long long target) {
    size_t lo = 0;
    size_t hi = values.size();
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (values[mid] < target) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    if (lo < values.size() && values[lo] == target) {
        return static_cast<long long>(lo);
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
