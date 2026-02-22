const fs = require("fs");
const readline = require("readline");

function lineLengthSum(path) {
  return new Promise((resolve, reject) => {
    let total = 0;
    const stream = fs.createReadStream(path, { encoding: "utf8" });
    const rl = readline.createInterface({
      input: stream,
      crlfDelay: Infinity,
    });

    rl.on("line", (line) => {
      total += line.length;
    });
    rl.on("close", () => {
      resolve(total);
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
  for (let i = 0; i < 12; i += 1) {
    checksum += (await lineLengthSum(path)) + (i % 4);
  }

  console.log(checksum);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
