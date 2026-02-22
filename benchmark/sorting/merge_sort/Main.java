class Main {
    public static void main(String[] args) {
        long[] values = new long[3201];
        values[0] = 0;
        for (int i = 0; i < 3200; i++) {
            values[i + 1] = (i * 29L + 13) % 20000;
        }

        int n = values.length;
        long[] temp = new long[n];
        for (int width = 1; width < n; width *= 2) {
            for (int left = 0; left < n; left += 2 * width) {
                int mid = Math.min(left + width, n);
                int right = Math.min(left + 2 * width, n);

                int i = left;
                int j = mid;
                int k = left;

                while (i < mid && j < right) {
                    if (values[i] <= values[j]) {
                        temp[k++] = values[i++];
                    } else {
                        temp[k++] = values[j++];
                    }
                }

                while (i < mid) {
                    temp[k++] = values[i++];
                }

                while (j < right) {
                    temp[k++] = values[j++];
                }
            }

            long[] swap = values;
            values = temp;
            temp = swap;
        }

        long checksum = 0;
        for (int i = 0; i < 128; i++) {
            checksum += values[i];
        }

        System.out.println(checksum);
    }
}
