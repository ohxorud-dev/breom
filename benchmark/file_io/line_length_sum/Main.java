import java.io.IOException;
import java.io.BufferedReader;
import java.io.FileReader;

class Main {
    private static long lineLengthSum(String path) {
        long total = 0;
        try (BufferedReader reader = new BufferedReader(new FileReader(path))) {
            String line;
            while ((line = reader.readLine()) != null) {
                total += line.length();
            }
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
        return total;
    }

    public static void main(String[] args) {
        String path = "benchmark/.tmp/io_fixture.txt";
        if (args.length > 0) {
            path = args[0];
        }

        long checksum = 0;
        for (int i = 0; i < 12; i++) {
            checksum += lineLengthSum(path) + (i % 4);
        }

        System.out.println(checksum);
    }
}
