const fs = require("fs");

function main() {
  const path = process.argv[2] || "benchmark/.tmp/io_fixture.txt";

  let checksum = 0;
  for (let i = 0; i < 20; i += 1) {
    const data = fs.readFileSync(path);
    checksum += data.length;
  }

  console.log(checksum);
}

main();
