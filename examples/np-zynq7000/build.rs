use network_partition_config::config::Config;
use network_partition_config::generate::generate_network_partition;
use quote::quote;
use std::env;
use std::ffi::OsString;
use std::fs::{read_to_string, write};
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap_or_default();
    let config_dir = env::var_os("CONFIG_DIR").unwrap_or(OsString::from("../../config/local_echo"));
    let dest_path = Path::new(&out_dir).join("np.rs");
    let config_path = Path::new(&config_dir).join("network_partition.yml");
    gen_config(config_path.as_path(), dest_path.as_path(), &out_dir);
    println!("cargo:rerun-if-changed=gen-config");
    println!("cargo:rerun-if-changed=config");
}

fn gen_config(config: &Path, dest: &Path, out_dir: &OsString) {
    let config = read_to_string(config).unwrap();
    let config: Config = serde_yaml::from_str(&config).unwrap();

    let network_partition = generate_network_partition(
        &config,
        quote!(apex_rs_xng::apex::XngHypervisor),
        quote!(network_partition_xng::network::UartSerial),
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
