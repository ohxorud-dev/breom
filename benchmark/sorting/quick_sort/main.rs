fn partition(values: &mut [i64], low: usize, high: usize) -> usize {
    let pivot = values[high];
    let mut i = low;
    for j in low..high {
        if values[j] <= pivot {
            values.swap(i, j);
            i += 1;
        }
    }
    values.swap(i, high);
    i
}

fn quick_sort(values: &mut [i64], low: usize, high: usize) {
    if low >= high {
        return;
    }

    let p = partition(values, low, high);
    if p > 0 {
        quick_sort(values, low, p - 1);
    }
    quick_sort(values, p + 1, high);
}

fn main() {
    let mut values = Vec::with_capacity(3201);
    values.push(0_i64);
    for i in 0_i64..3200 {
        values.push((i * 29 + 13) % 20_000);
    }

    if !values.is_empty() {
        let last = values.len() - 1;
        quick_sort(&mut values, 0, last);
    }

    let checksum: i64 = values.iter().take(128).sum();
    println!("{checksum}");
}
