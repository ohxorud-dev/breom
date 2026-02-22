fn main() {
    let mut values = Vec::with_capacity(200_001);
    values.push(0_i64);
    for i in 0_i64..200_000 {
        values.push((i * 9 + 5) % 1_000);
    }

    let mut checksum = 0_i64;
    for round in 0_i64..40 {
        let mut local = 0_i64;
        for v in &values {
            local += *v + (round % 3);
        }
        checksum += local;
    }

    println!("{checksum}");
}
