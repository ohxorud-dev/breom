#include <iostream>
#include <vector>

int main() {
    std::vector<long long> values;
    values.reserve(3201);
    values.push_back(0);
    for (long long i = 0; i < 3200; ++i) {
        values.push_back((i * 29 + 13) % 20000);
    }

    size_t n = values.size();
    std::vector<long long> temp(n);
    for (size_t width = 1; width < n; width *= 2) {
        for (size_t left = 0; left < n; left += 2 * width) {
            size_t mid = left + width;
            if (mid > n) {
                mid = n;
            }

            size_t right = left + 2 * width;
            if (right > n) {
                right = n;
            }

            size_t i = left;
            size_t j = mid;
            size_t k = left;

            while (i < mid && j < right) {
                if (values[i] <= values[j]) {
                    temp[k++] = values[i++];
                } else {
                    temp[k++] = values[j++];
                }
            }

            while (i < mid) {
                temp[k++] = values[i++];
            }

            while (j < right) {
                temp[k++] = values[j++];
            }
        }

        values.swap(temp);
    }

    long long checksum = 0;
    for (long long i = 0; i < 128; ++i) {
        checksum += values[i];
    }

    std::cout << checksum << '\n';
    return 0;
}
