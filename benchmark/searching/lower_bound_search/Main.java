class Main {
    private static int searchIndex(long[] values, long target) {
        int lo = 0;
        int hi = values.length;
        while (lo < hi) {
            int mid = (lo + hi) / 2;
            if (values[mid] < target) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if (lo < values.length && values[lo] == target) {
            return lo;
        }
        return -1;
    }

    public static void main(String[] args) {
        long[] values = new long[50001];
        values[0] = 0;
        for (int i = 0; i < 50000; i++) {
            values[i + 1] = i * 2L;
        }

        long target = 49991L * 2;
        long checksum = 0;
        for (int i = 0; i < 420000; i++) {
            int idx = searchIndex(values, target);
            checksum += (idx % 97) + (i % 5);
        }

        System.out.println(checksum);
    }
}
