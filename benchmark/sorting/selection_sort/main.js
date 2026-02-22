function main() {
  const values = [0];
  for (let i = 0; i < 3200; i += 1) {
    values.push((i * 29 + 13) % 20000);
  }

  for (let i = 0; i < values.length; i += 1) {
    let minIndex = i;
    for (let j = i + 1; j < values.length; j += 1) {
      if (values[j] < values[minIndex]) {
        minIndex = j;
      }
    }

    if (minIndex !== i) {
      [values[i], values[minIndex]] = [values[minIndex], values[i]];
    }
  }

  let checksum = 0;
  for (let i = 0; i < 128; i += 1) {
    checksum += values[i];
  }

  console.log(checksum);
}

main();
