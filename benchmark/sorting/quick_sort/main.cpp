#include <iostream>
#include <vector>

size_t partition(std::vector<long long>& values, size_t low, size_t high) {
    long long pivot = values[high];
    size_t i = low;
    for (size_t j = low; j < high; ++j) {
        if (values[j] <= pivot) {
            std::swap(values[i], values[j]);
            ++i;
        }
    }
    std::swap(values[i], values[high]);
    return i;
}

void quick_sort(std::vector<long long>& values, size_t low, size_t high) {
    if (low >= high) {
        return;
    }

    size_t p = partition(values, low, high);
    if (p > 0) {
        quick_sort(values, low, p - 1);
    }
    quick_sort(values, p + 1, high);
}

int main() {
    std::vector<long long> values;
    values.reserve(3201);
    values.push_back(0);
    for (long long i = 0; i < 3200; ++i) {
        values.push_back((i * 29 + 13) % 20000);
    }

    if (!values.empty()) {
        quick_sort(values, 0, values.size() - 1);
    }

    long long checksum = 0;
    for (long long i = 0; i < 128; ++i) {
        checksum += values[i];
    }

    std::cout << checksum << '\n';
    return 0;
}
