function main() {
  const values = [0];
  for (let i = 0; i < 2200; i += 1) {
    values.push((i * 17 + 23) % 10000);
  }

  const n = values.length;
  for (let i = 0; i < n; i += 1) {
    const limit = n - i - 1;
    for (let j = 0; j < limit; j += 1) {
      if (values[j] > values[j + 1]) {
        const tmp = values[j];
        values[j] = values[j + 1];
        values[j + 1] = tmp;
      }
    }
  }

  let checksum = 0;
  for (let i = 0; i < 128; i += 1) {
    checksum += values[i];
  }

  console.log(checksum);
}

main();
