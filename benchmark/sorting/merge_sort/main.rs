fn main() {
    let mut values = Vec::with_capacity(3201);
    values.push(0_i64);
    for i in 0_i64..3200 {
        values.push((i * 29 + 13) % 20_000);
    }

    let n = values.len();
    let mut temp = vec![0_i64; n];
    let mut width = 1;
    while width < n {
        let mut left = 0;
        while left < n {
            let mid = (left + width).min(n);
            let right = (left + 2 * width).min(n);

            let mut i = left;
            let mut j = mid;
            let mut k = left;

            while i < mid && j < right {
                if values[i] <= values[j] {
                    temp[k] = values[i];
                    i += 1;
                } else {
                    temp[k] = values[j];
                    j += 1;
                }
                k += 1;
            }

            while i < mid {
                temp[k] = values[i];
                i += 1;
                k += 1;
            }

            while j < right {
                temp[k] = values[j];
                j += 1;
                k += 1;
            }

            left += 2 * width;
        }

        values.copy_from_slice(&temp);
        width *= 2;
    }

    let checksum: i64 = values.iter().take(128).sum();
    println!("{checksum}");
}
