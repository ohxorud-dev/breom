class Main {
    public static void main(String[] args) {
        long[] values = new long[3201];
        values[0] = 0;
        for (int i = 0; i < 3200; i++) {
            values[i + 1] = (i * 29L + 13) % 20000;
        }

        for (int i = 0; i < values.length; i++) {
            int minIndex = i;
            for (int j = i + 1; j < values.length; j++) {
                if (values[j] < values[minIndex]) {
                    minIndex = j;
                }
            }

            if (minIndex != i) {
                long tmp = values[i];
                values[i] = values[minIndex];
                values[minIndex] = tmp;
            }
        }

        long checksum = 0;
        for (int i = 0; i < 128; i++) {
            checksum += values[i];
        }

        System.out.println(checksum);
    }
}
