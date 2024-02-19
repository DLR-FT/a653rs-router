use a653rs_router::prelude::RouterConfig;
use std::io::{stdin, stdout, BufReader, BufWriter, Write};

fn main() {
    let cfg = BufReader::new(stdin());
    let cfg: RouterConfig<10, 10, 10, 10> =
        serde_yaml::from_reader(cfg).expect("Failed to read config");
    let cfg: heapless::Vec<u8, 10_000> =
        postcard::to_vec(&cfg).expect("Failed to serialize config");
    let mut out = BufWriter::new(stdout());
    out.write_all(&cfg)
        .expect("Failed to write configuration binary blob")
}
