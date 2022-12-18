use network_partition_config::generate::generate_network_partition;
use quote::format_ident;
use quote::quote;
use std::env;
use std::ffi::OsString;
use std::fs::{read_to_string, write};
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap_or_default();
    let dest_path = Path::new(&out_dir).join("config.rs");
    let config_dir = env::var_os("CONFIG_DIR").unwrap_or(OsString::from("config"));
    let config_path = Path::new(&config_dir).join("network_partition_config.yml");
    let config = read_to_string(config_path).unwrap();

    let partition = format_ident!("NetworkPartition");
    let network_partition = generate_network_partition(
        serde_yaml::from_str(&config).unwrap(),
        format_ident!("ApexLinuxPartition"),
        partition.clone(),
    );

    let init = quote! {
        use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};

        #network_partition

        fn main() {
            ApexLogger::install_panic_hook();
            ApexLogger::install_logger(LevelFilter::Trace).unwrap();
            #partition.run();
        }
    };

    write(&dest_path, init.to_string()).unwrap();

    // format the generated source code
    if let Err(e) = Command::new("rustfmt")
        .arg(dest_path.as_os_str())
        .current_dir(&out_dir)
        .status()
    {
        eprintln!("{e}")
    }

    println!("cargo:rerun-if-changed=config");
}
