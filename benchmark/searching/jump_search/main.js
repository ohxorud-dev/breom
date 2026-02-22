function searchIndex(values, target) {
  const n = values.length;
  if (n === 0) {
    return -1;
  }

  const jump = Math.max(1, Math.floor(Math.sqrt(n)));
  let step = jump;
  let prev = 0;

  while (prev < n) {
    const blockEnd = step < n ? step : n;
    if (values[blockEnd - 1] >= target) {
      break;
    }
    prev = step;
    step += jump;
    if (prev >= n) {
      return -1;
    }
  }

  const blockEnd = step < n ? step : n;
  for (let i = prev; i < blockEnd; i += 1) {
    const v = values[i];
    if (v === target) {
      return i;
    }
    if (v > target) {
      break;
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
