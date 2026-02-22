class Main {
    public static void main(String[] args) {
        long[] values = new long[180001];
        values[0] = 0;
        for (int i = 0; i < 180000; i++) {
            values[i + 1] = (i * 11L + 3) % 1000;
        }

        long checksum = 0;
        for (int round = 0; round < 24; round++) {
            long running = 0;
            long local = 0;
            for (long v : values) {
                running += v;
                local += (running % 1000) + (round % 7);
            }
            checksum += local;
        }

        System.out.println(checksum);
    }
}
