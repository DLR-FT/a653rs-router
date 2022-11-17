extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use log::{error, trace, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;

// TODO should be configured from config using proc-macro
const ECHO_PORT_SIZE_BYTES: MessageSize = 10000;
static ECHO_SEND: OnceCell<SamplingPortSource<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>> =
    OnceCell::new();
static ECHO_RECV: OnceCell<SamplingPortDestination<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>> =
    OnceCell::new();

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
    let config = include_str!("../../config/network_partition_config.yml");
    let parsed_config = serde_yaml::from_str::<Config>(config);
    if let Err(error) = parsed_config {
        error!("{error:?}");
        panic!();
    }
    let config = parsed_config.ok().unwrap();
    trace!("Have config: {config:?}");
    let partition = NetworkPartition::<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>::new(
        config,
        &ECHO_RECV,
        &ECHO_SEND,
        entry_point,
    );
    partition.run()
}

extern "C" fn entry_point() {
    let input = ECHO_RECV.get().unwrap();
    let output = ECHO_SEND.get().unwrap();
    run::<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>(input, output);
}

#[cfg(test)]
mod tests {
    use network_partition::prelude::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn parse_code_section_config() {
        // TODO should be configured from config using proc-macro
        let config = include_str!("../../config/network_partition_config.yml");
        let parsed = serde_yaml::from_str::<Config>(config);
        assert!(parsed.is_ok());
    }
}
