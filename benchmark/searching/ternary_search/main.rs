fn search_index(values: &[i64], target: i64) -> i64 {
    let mut lo: i64 = 0;
    let mut hi: i64 = values.len() as i64 - 1;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        let v = values[mid as usize];
        if v == target {
            return mid;
        }
        if v < target {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    -1
}

fn main() {
    let mut values = Vec::with_capacity(50001);
    values.push(0_i64);
    for i in 0_i64..50000 {
        values.push(i * 2);
    }

    let target = 49_991_i64 * 2;
    let mut checksum = 0_i64;
    for i in 0_i64..420_000 {
        let idx = search_index(&values, target);
        checksum += (idx % 97) + (i % 5);
    }

    println!("{checksum}");
}
