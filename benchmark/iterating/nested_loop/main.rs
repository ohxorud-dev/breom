fn main() {
    let mut checksum = 0_i64;

    for outer in 0_i64..300 {
        let mut row = 0_i64;
        for inner in 0_i64..4000 {
            row += ((outer + 1) * (inner + 3)) % 97;
        }
        checksum += row;
    }

    println!("{checksum}");
}
