class Main {
    private static int partition(long[] values, int low, int high) {
        long pivot = values[high];
        int i = low;
        for (int j = low; j < high; j++) {
            if (values[j] <= pivot) {
                long tmp = values[i];
                values[i] = values[j];
                values[j] = tmp;
                i++;
            }
        }
        long tmp = values[i];
        values[i] = values[high];
        values[high] = tmp;
        return i;
    }

    private static void quickSort(long[] values, int low, int high) {
        if (low >= high) {
            return;
        }

        int pivotIndex = partition(values, low, high);
        if (pivotIndex > 0) {
            quickSort(values, low, pivotIndex - 1);
        }
        quickSort(values, pivotIndex + 1, high);
    }

    public static void main(String[] args) {
        long[] values = new long[3201];
        values[0] = 0;
        for (int i = 0; i < 3200; i++) {
            values[i + 1] = (i * 29L + 13) % 20000;
        }

        if (values.length > 0) {
            quickSort(values, 0, values.length - 1);
        }

        long checksum = 0;
        for (int i = 0; i < 128; i++) {
            checksum += values[i];
        }

        System.out.println(checksum);
    }
}
