use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn line_length_sum(path: &str) -> u64 {
    let file = File::open(path).expect("failed to open fixture");
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut total = 0_u64;

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).expect("failed to read fixture");
        if bytes == 0 {
            break;
        }
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        total += line.len() as u64;
    }

    total
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("benchmark/.tmp/io_fixture.txt");

    let mut checksum: u64 = 0;
    for i in 0_u64..12 {
        checksum += line_length_sum(path) + (i % 4);
    }

    println!("{checksum}");
}
