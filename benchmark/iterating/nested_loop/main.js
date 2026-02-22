function main() {
  let checksum = 0;

  for (let outer = 0; outer < 300; outer += 1) {
    let row = 0;
    for (let inner = 0; inner < 4000; inner += 1) {
      row += ((outer + 1) * (inner + 3)) % 97;
    }
    checksum += row;
  }

  console.log(checksum);
}

main();
