import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

class Main {
    public static void main(String[] args) {
        String path = "benchmark/.tmp/io_fixture.txt";
        if (args.length > 0) {
            path = args[0];
        }

        long checksum = 0;
        for (int i = 0; i < 20; i++) {
            byte[] data;
            try {
                data = Files.readAllBytes(Path.of(path));
            } catch (IOException e) {
                throw new RuntimeException(e);
            }
            checksum += data.length;
        }

        System.out.println(checksum);
    }
}
