function main() {
  const values = [0];
  for (let i = 0; i < 3200; i += 1) {
    values.push((i * 29 + 13) % 20000);
  }

  const n = values.length;
  const temp = new Array(n);
  for (let width = 1; width < n; width *= 2) {
    for (let left = 0; left < n; left += 2 * width) {
      const mid = Math.min(left + width, n);
      const right = Math.min(left + 2 * width, n);

      let i = left;
      let j = mid;
      let k = left;

      while (i < mid && j < right) {
        if (values[i] <= values[j]) {
          temp[k] = values[i];
          i += 1;
        } else {
          temp[k] = values[j];
          j += 1;
        }
        k += 1;
      }

      while (i < mid) {
        temp[k] = values[i];
        i += 1;
        k += 1;
      }

      while (j < right) {
        temp[k] = values[j];
        j += 1;
        k += 1;
      }
    }

    for (let idx = 0; idx < n; idx += 1) {
      values[idx] = temp[idx];
    }
  }

  let checksum = 0;
  for (let i = 0; i < 128; i += 1) {
    checksum += values[i];
  }

  console.log(checksum);
}

main();
