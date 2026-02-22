class Main {
    public static void main(String[] args) {
        long[] values = new long[3201];
        values[0] = 0;
        for (int i = 0; i < 3200; i++) {
            values[i + 1] = (i * 29L + 13) % 20000;
        }

        for (int i = 1; i < values.length; i++) {
            long key = values[i];
            int j = i;
            while (j > 0 && values[j - 1] > key) {
                values[j] = values[j - 1];
                j--;
            }
            values[j] = key;
        }

        long checksum = 0;
        for (int i = 0; i < 128; i++) {
            checksum += values[i];
        }

        System.out.println(checksum);
    }
}
