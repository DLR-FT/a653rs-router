use network_partition_config::generate::generate_network_partition;
use quote::quote;
use std::env;
use std::ffi::OsString;
use std::fs::{read_to_string, write};
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap_or_default();
    let config_dir = env::var_os("CONFIG_DIR").unwrap_or(OsString::from("config"));

    {
        let dest_path = Path::new(&out_dir).join("np-client.rs");
        let config_path = Path::new(&config_dir).join("np-client.yml");
        gen_config(config_path.as_path(), dest_path.as_path(), &out_dir);
    }

    {
        let dest_path = Path::new(&out_dir).join("np-server.rs");
        let config_path = Path::new(&config_dir).join("np-server.yml");
        gen_config(config_path.as_path(), dest_path.as_path(), &out_dir);
    }

    println!("cargo:rerun-if-changed=config");
}

fn gen_config(config: &Path, dest: &Path, out_dir: &OsString) {
    let config = read_to_string(config).unwrap();

    let network_partition = generate_network_partition(
        serde_yaml::from_str(&config).unwrap(),
        quote!(apex_rs_linux::partition::ApexLinuxPartition),
        quote!(crate::UdpInterface),
    );

    write(dest, network_partition.to_string()).unwrap();

    // format the generated source code
    if let Err(e) = Command::new("rustfmt")
        .arg(dest.as_os_str())
        .current_dir(out_dir)
        .status()
    {
        eprintln!("{e}")
    }
}
