fn main() {
    let mut values = Vec::with_capacity(180001);
    values.push(0_i64);
    for i in 0_i64..180_000 {
        values.push((i * 11 + 3) % 1000);
    }

    let mut checksum = 0_i64;
    for round in 0_i64..24 {
        let mut running = 0_i64;
        let mut local = 0_i64;
        for &v in &values {
            running += v;
            local += (running % 1000) + (round % 7);
        }
        checksum += local;
    }

    println!("{checksum}");
}
