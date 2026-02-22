#include <iostream>

int main() {
    long long checksum = 0;

    for (long long outer = 0; outer < 300; ++outer) {
        long long row = 0;
        for (long long inner = 0; inner < 4000; ++inner) {
            row += ((outer + 1) * (inner + 3)) % 97;
        }
        checksum += row;
    }

    std::cout << checksum << '\n';
    return 0;
}
