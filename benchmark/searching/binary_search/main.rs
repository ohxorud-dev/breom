fn binary_search(values: &[i64], target: i64) -> i64 {
    let mut lo: i64 = 0;
    let mut hi: i64 = values.len() as i64 - 1;

    while lo <= hi {
        let mid = (lo + hi) / 2;
        let mid_v = values[mid as usize];
        if mid_v == target {
            return mid;
        }
        if mid_v < target {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }

    -1
}

fn main() {
    let mut values = Vec::with_capacity(40_001);
    values.push(0_i64);
    for i in 0_i64..40_000 {
        values.push(i * 2);
    }

    let target = 39_993 * 2;
    let mut checksum = 0_i64;
    for i in 0_i64..500_000 {
        let idx = binary_search(&values, target);
        checksum += (idx % 97) + (i % 3);
    }

    println!("{checksum}");
}
