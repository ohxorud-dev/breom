function linearSearch(values, target) {
  for (let i = 0; i < values.length; i += 1) {
    if (values[i] === target) {
      return i;
    }
  }
  return -1;
}

function main() {
  const values = [0];
  for (let i = 0; i < 20000; i += 1) {
    values.push(i * 3);
  }

  const target = (values.length - 7) * 3;
  let checksum = 0;
  for (let i = 0; i < 6000; i += 1) {
    const idx = linearSearch(values, target);
    checksum += idx + (i % 5);
  }

  console.log(checksum);
}

main();
