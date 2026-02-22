class Main {
    public static void main(String[] args) {
        int[] values = new int[260001];
        values[0] = 0;
        for (int i = 0; i < 260000; i++) {
            values[i + 1] = (i * 17 + 5) % 256;
        }

        long checksum = 0;
        for (int round = 0; round < 32; round++) {
            long[] counts = new long[256];
            for (int v : values) {
                counts[v]++;
            }

            long local = 0;
            for (int i = 0; i < 256; i++) {
                local += counts[i] * (i + 1L + (round % 3));
            }
            checksum += local;
        }

        System.out.println(checksum);
    }
}
