class Main {
    public static void main(String[] args) {
        long checksum = 0;

        for (long outer = 0; outer < 300; outer++) {
            long row = 0;
            for (long inner = 0; inner < 4000; inner++) {
                row += ((outer + 1) * (inner + 3)) % 97;
            }
            checksum += row;
        }

        System.out.println(checksum);
    }
}
