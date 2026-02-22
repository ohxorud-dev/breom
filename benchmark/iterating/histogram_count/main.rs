fn main() {
    let mut values = Vec::with_capacity(260001);
    values.push(0_usize);
    for i in 0_usize..260_000 {
        values.push((i * 17 + 5) % 256);
    }

    let mut checksum: u64 = 0;
    for round in 0_u64..32 {
        let mut counts = [0_u64; 256];
        for &v in &values {
            counts[v] += 1;
        }

        let mut local = 0_u64;
        for i in 0..256_u64 {
            local += counts[i as usize] * (i + 1 + (round % 3));
        }
        checksum += local;
    }

    println!("{checksum}");
}
