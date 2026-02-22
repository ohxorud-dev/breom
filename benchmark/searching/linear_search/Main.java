class Main {
    private static long linearSearch(long[] values, long target) {
        for (int i = 0; i < values.length; i++) {
            if (values[i] == target) {
                return i;
            }
        }
        return -1;
    }

    public static void main(String[] args) {
        long[] values = new long[20001];
        values[0] = 0;
        for (int i = 0; i < 20000; i++) {
            values[i + 1] = i * 3L;
        }

        long target = (values.length - 7L) * 3;
        long checksum = 0;
        for (long i = 0; i < 6000; i++) {
            long idx = linearSearch(values, target);
            checksum += idx + (i % 5);
        }

        System.out.println(checksum);
    }
}
