use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn count_lines(path: &str) -> i64 {
    let file = File::open(path).expect("failed to open fixture");
    let reader = BufReader::new(file);
    reader.lines().count() as i64
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("benchmark/.tmp/io_fixture.txt");

    let mut checksum = 0_i64;
    for i in 0_i64..15 {
        checksum += count_lines(path) + (i % 3);
    }

    println!("{checksum}");
}
