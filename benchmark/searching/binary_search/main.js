function binarySearch(values, target) {
  let lo = 0;
  let hi = values.length - 1;
  while (lo <= hi) {
    const mid = Math.floor((lo + hi) / 2);
    const midV = values[mid];
    if (midV === target) {
      return mid;
    }
    if (midV < target) {
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return -1;
}

function main() {
  const values = [0];
  for (let i = 0; i < 40000; i += 1) {
    values.push(i * 2);
  }

  const target = 39993 * 2;
  let checksum = 0;
  for (let i = 0; i < 500000; i += 1) {
    const idx = binarySearch(values, target);
    checksum += (idx % 97) + (i % 3);
  }

  console.log(checksum);
}

main();
