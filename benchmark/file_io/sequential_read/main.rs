use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("benchmark/.tmp/io_fixture.txt");

    let mut checksum = 0_u64;
    for _ in 0..20 {
        let data = fs::read(path).expect("failed to read fixture");
        checksum += data.len() as u64;
    }

    println!("{checksum}");
}
