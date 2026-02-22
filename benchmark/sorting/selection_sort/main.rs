fn main() {
    let mut values = Vec::with_capacity(3201);
    values.push(0_i64);
    for i in 0_i64..3200 {
        values.push((i * 29 + 13) % 20_000);
    }

    for i in 0..values.len() {
        let mut min_index = i;
        for j in (i + 1)..values.len() {
            if values[j] < values[min_index] {
                min_index = j;
            }
        }

        if min_index != i {
            values.swap(i, min_index);
        }
    }

    let checksum: i64 = values.iter().take(128).sum();
    println!("{checksum}");
}
