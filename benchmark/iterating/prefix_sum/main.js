function main() {
  const values = [0];
  for (let i = 0; i < 180000; i += 1) {
    values.push((i * 11 + 3) % 1000);
  }

  let checksum = 0;
  for (let round = 0; round < 24; round += 1) {
    let running = 0;
    let local = 0;
    for (let i = 0; i < values.length; i += 1) {
      running += values[i];
      local += (running % 1000) + (round % 7);
    }
    checksum += local;
  }

  console.log(checksum);
}

main();
