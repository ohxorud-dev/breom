function partition(values, low, high) {
  const pivot = values[high];
  let i = low;
  for (let j = low; j < high; j += 1) {
    if (values[j] <= pivot) {
      [values[i], values[j]] = [values[j], values[i]];
      i += 1;
    }
  }
  [values[i], values[high]] = [values[high], values[i]];
  return i;
}

function quickSort(values, low, high) {
  if (low >= high) {
    return;
  }

  const pivotIndex = partition(values, low, high);
  if (pivotIndex > 0) {
    quickSort(values, low, pivotIndex - 1);
  }
  quickSort(values, pivotIndex + 1, high);
}

function main() {
  const values = [0];
  for (let i = 0; i < 3200; i += 1) {
    values.push((i * 29 + 13) % 20000);
  }

  if (values.length > 0) {
    quickSort(values, 0, values.length - 1);
  }

  let checksum = 0;
  for (let i = 0; i < 128; i += 1) {
    checksum += values[i];
  }

  console.log(checksum);
}

main();
