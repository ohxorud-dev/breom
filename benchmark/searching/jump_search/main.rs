fn search_index(values: &[i64], target: i64) -> i64 {
    let n = values.len();
    if n == 0 {
        return -1;
    }

    let jump = (n as f64).sqrt() as usize;
    let mut step = if jump == 0 { 1 } else { jump };
    let mut prev = 0_usize;

    while prev < n {
        let block_end = if step < n { step } else { n };
        if values[block_end - 1] >= target {
            break;
        }
        prev = step;
        step += jump;
        if prev >= n {
            return -1;
        }
    }

    let block_end = if step < n { step } else { n };
    for i in prev..block_end {
        let v = values[i];
        if v == target {
            return i as i64;
        }
        if v > target {
            break;
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
