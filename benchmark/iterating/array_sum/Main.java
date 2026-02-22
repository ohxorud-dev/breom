class Main {
    public static void main(String[] args) {
        long[] values = new long[200001];
        values[0] = 0;
        for (int i = 0; i < 200000; i++) {
            values[i + 1] = (i * 9L + 5) % 1000;
        }

        long checksum = 0;
        for (long round = 0; round < 40; round++) {
            long local = 0;
            for (long v : values) {
                local += v + (round % 3);
            }
            checksum += local;
        }

        System.out.println(checksum);
    }
}
