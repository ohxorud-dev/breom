class Main {
    private static int searchIndex(long[] values, long target) {
        int n = values.length;
        if (n == 0) {
            return -1;
        }

        int jump = (int) Math.sqrt(n);
        if (jump == 0) {
            jump = 1;
        }
        int step = jump;
        int prev = 0;

        while (prev < n) {
            int blockEnd = step < n ? step : n;
            if (values[blockEnd - 1] >= target) {
                break;
            }
            prev = step;
            step += jump;
            if (prev >= n) {
                return -1;
            }
        }

        int blockEnd = step < n ? step : n;
        for (int i = prev; i < blockEnd; i++) {
            long v = values[i];
            if (v == target) {
                return i;
            }
            if (v > target) {
                break;
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
