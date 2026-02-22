function main() {
  const values = [0];
  for (let i = 0; i < 3200; i += 1) {
    values.push((i * 29 + 13) % 20000);
  }

  for (let i = 1; i < values.length; i += 1) {
    const key = values[i];
    let j = i;
    while (j > 0 && values[j - 1] > key) {
      values[j] = values[j - 1];
      j -= 1;
    }
    values[j] = key;
  }

  let checksum = 0;
  for (let i = 0; i < 128; i += 1) {
    checksum += values[i];
  }

  console.log(checksum);
}

main();
