class Main {
    private static int searchIndex(long[] values, long target) {
        if (values.length == 0) {
            return -1;
        }
        if (values[0] == target) {
            return 0;
        }

        int bound = 1;
        while (bound < values.length && values[bound] < target) {
            bound *= 2;
        }

        int lo = bound / 2;
        int hi = Math.min(bound, values.length - 1);
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            long v = values[mid];
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
