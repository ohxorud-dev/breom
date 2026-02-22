import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;

class Main {
    private static long countLines(String path) {
        long count = 0;
        try (BufferedReader reader = new BufferedReader(new FileReader(path))) {
            while (reader.readLine() != null) {
                count++;
            }
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
        return count;
    }

    public static void main(String[] args) {
        String path = "benchmark/.tmp/io_fixture.txt";
        if (args.length > 0) {
            path = args[0];
        }

        long checksum = 0;
        for (long i = 0; i < 15; i++) {
            checksum += countLines(path) + (i % 3);
        }

        System.out.println(checksum);
    }
}
