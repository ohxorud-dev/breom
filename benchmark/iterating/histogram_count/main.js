function main() {
  const values = [0];
  for (let i = 0; i < 260000; i += 1) {
    values.push((i * 17 + 5) % 256);
  }

  let checksum = 0;
  for (let round = 0; round < 32; round += 1) {
    const counts = new Array(256).fill(0);
    for (let i = 0; i < values.length; i += 1) {
      counts[values[i]] += 1;
    }

    let local = 0;
    for (let i = 0; i < 256; i += 1) {
      local += counts[i] * (i + 1 + (round % 3));
    }
    checksum += local;
  }

  console.log(checksum);
}

main();
