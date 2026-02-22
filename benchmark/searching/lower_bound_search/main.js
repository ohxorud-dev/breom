function searchIndex(values, target) {
  let lo = 0;
  let hi = values.length;
  while (lo < hi) {
    const mid = Math.floor((lo + hi) / 2);
    if (values[mid] < target) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  if (lo < values.length && values[lo] === target) {
    return lo;
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
