fn main() {
    let mut values = Vec::with_capacity(3201);
    values.push(0_i64);
    for i in 0_i64..3200 {
        values.push((i * 29 + 13) % 20_000);
    }

    for i in 1..values.len() {
        let key = values[i];
        let mut j = i;
        while j > 0 && values[j - 1] > key {
            values[j] = values[j - 1];
            j -= 1;
        }
        values[j] = key;
    }

    let checksum: i64 = values.iter().take(128).sum();
    println!("{checksum}");
}
