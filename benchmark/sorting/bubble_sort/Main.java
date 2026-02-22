class Main {
    public static void main(String[] args) {
        long[] values = new long[2201];
        values[0] = 0;
        for (int i = 0; i < 2200; i++) {
            values[i + 1] = (i * 17L + 23) % 10000;
        }

        int n = values.length;
        for (int i = 0; i < n; i++) {
            int limit = n - i - 1;
            for (int j = 0; j < limit; j++) {
                if (values[j] > values[j + 1]) {
                    long tmp = values[j];
                    values[j] = values[j + 1];
                    values[j + 1] = tmp;
                }
            }
        }

        long checksum = 0;
        for (int i = 0; i < 128; i++) {
            checksum += values[i];
        }

        System.out.println(checksum);
    }
}
