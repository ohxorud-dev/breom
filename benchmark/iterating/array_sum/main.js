function main() {
  const values = [0];
  for (let i = 0; i < 200000; i += 1) {
    values.push((i * 9 + 5) % 1000);
  }

  let checksum = 0;
  for (let round = 0; round < 40; round += 1) {
    let local = 0;
    for (let i = 0; i < values.length; i += 1) {
      local += values[i] + (round % 3);
    }
    checksum += local;
  }

  console.log(checksum);
}

main();
