const fs = require("fs");
const readline = require("readline");

function countLines(path) {
  return new Promise((resolve, reject) => {
    let count = 0;
    const stream = fs.createReadStream(path, { encoding: "utf8" });
    const rl = readline.createInterface({
      input: stream,
      crlfDelay: Infinity,
    });

    rl.on("line", () => {
      count += 1;
    });
    rl.on("close", () => {
      resolve(count);
    });
    rl.on("error", (err) => {
      reject(err);
    });
    stream.on("error", (err) => {
      reject(err);
    });
  });
}

async function main() {
  const path = process.argv[2] || "benchmark/.tmp/io_fixture.txt";

  let checksum = 0;
  for (let i = 0; i < 15; i += 1) {
    checksum += (await countLines(path)) + (i % 3);
  }

  console.log(checksum);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
