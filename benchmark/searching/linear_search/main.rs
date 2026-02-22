fn linear_search(values: &[i64], target: i64) -> i64 {
    for (i, v) in values.iter().enumerate() {
        if *v == target {
            return i as i64;
        }
    }
    -1
}

fn main() {
    let mut values = Vec::with_capacity(20_001);
    values.push(0_i64);
    for i in 0_i64..20_000 {
        values.push(i * 3);
    }

    let target = (values.len() as i64 - 7) * 3;
    let mut checksum = 0_i64;
    for i in 0_i64..6_000 {
        let idx = linear_search(&values, target);
        checksum += idx + (i % 5);
    }

    println!("{checksum}");
}
