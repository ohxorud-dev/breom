#include <iostream>
#include <vector>

long long search_index(const std::vector<long long>& values, long long target) {
    if (values.empty()) {
        return -1;
    }
    if (values[0] == target) {
        return 0;
    }

    long long bound = 1;
    const long long n = static_cast<long long>(values.size());
    while (bound < n && values[static_cast<size_t>(bound)] < target) {
        bound *= 2;
    }

    long long lo = bound / 2;
    long long hi = bound < n ? bound : n - 1;
    while (lo <= hi) {
        long long mid = (lo + hi) / 2;
        long long v = values[static_cast<size_t>(mid)];
        if (v == target) {
            return mid;
        }
        if (v < target) {
            lo = mid + 1;
        } else {
            hi = mid - 1;
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
