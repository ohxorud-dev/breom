function searchIndex(values, target) {
  let lo = 0;
  let hi = values.length - 1;
  while (lo <= hi) {
    const mid = Math.floor((lo + hi) / 2);
    const v = values[mid];
    if (v === target) {
      return mid;
    }
    if (v < target) {
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return -1;
}

function main() {
  const values = [0];
  for (let i = 0; i < 50000; i += 1) {
    values.push(i * 2);
  }

  const target = 49991 * 2;
  let checksum = 0;
  for (let i = 0; i < 420000; i += 1) {
    const idx = searchIndex(values, target);
    checksum += (idx % 97) + (i % 5);
  }

  console.log(checksum);
}

main();
