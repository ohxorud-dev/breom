class Main {
    private static long binarySearch(long[] values, long target) {
        long lo = 0;
        long hi = values.length - 1L;

        while (lo <= hi) {
            long mid = (lo + hi) / 2;
            long midV = values[(int) mid];
            if (midV == target) {
                return mid;
            }
            if (midV < target) {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        return -1;
    }

    public static void main(String[] args) {
        long[] values = new long[40001];
        values[0] = 0;
        for (int i = 0; i < 40000; i++) {
            values[i + 1] = i * 2L;
        }

        long target = 39993L * 2;
        long checksum = 0;
        for (long i = 0; i < 500000; i++) {
            long idx = binarySearch(values, target);
            checksum += (idx % 97) + (i % 3);
        }

        System.out.println(checksum);
    }
}
