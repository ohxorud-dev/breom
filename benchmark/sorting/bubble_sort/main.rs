fn main() {
    let mut values = Vec::with_capacity(2201);
    values.push(0_i64);
    for i in 0_i64..2200 {
        values.push((i * 17 + 23) % 10_000);
    }

    let n = values.len();
    for i in 0..n {
        let limit = n - i - 1;
        for j in 0..limit {
            if values[j] > values[j + 1] {
                values.swap(j, j + 1);
            }
        }
    }

    let checksum: i64 = values.iter().take(128).sum();
    println!("{checksum}");
}
