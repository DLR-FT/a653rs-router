use a653rs_router::prelude::Config;
use heapless::Vec;
use std::io::{stdin, stdout, BufReader, BufWriter, Write};

fn main() {
    let cfg = BufReader::new(stdin());
    let cfg: Config<10, 10> = serde_json::from_reader(cfg).expect("Failed to read config");
    let cfg: Vec<u8, 10_000> = postcard::to_vec(&cfg).expect("Failed to serialize config");
    let mut out = BufWriter::new(stdout());
    out.write_all(&cfg)
        .expect("Failed to write configuration binary blob")
}
