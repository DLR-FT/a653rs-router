extern crate log;

use apex_rs::prelude::*;
use apex_rs_linux::partition::{ApexLinuxPartition, ApexLogger};
use log::{error, trace, LevelFilter};
use network_partition::prelude::*;
use once_cell::sync::OnceCell;
use std::str::FromStr;

// TODO should be configured from config using proc-macro
const ECHO_PORT_SIZE_BYTES: MessageSize = 10000;
static CONFIG: OnceCell<Config> = OnceCell::new();
static ROUTER: OnceCell<RouterP4<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>> = OnceCell::new();

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
    let config = include_str!("../../config/network_partition_config.yml");
    let parsed_config = serde_yaml::from_str::<Config>(config);
    if let Err(error) = parsed_config {
        error!("{error:?}");
        panic!();
    }
    CONFIG.set(parsed_config.ok().unwrap()).unwrap();
    trace!("Have config: {CONFIG:?}");
    let partition = NetworkPartition::<ECHO_PORT_SIZE_BYTES, ApexLinuxPartition>::new(
        CONFIG.get().unwrap().clone(),
        &ROUTER,
        entry_point,
    );
    partition.run();
}

extern "C" fn entry_point() {
    let input = ChannelName::from_str("EchoRequest").unwrap();
    let output = ChannelName::from_str("EchoReply").unwrap();
    let router = ROUTER.get().unwrap();
    loop {
        let result = router.echo::<ECHO_PORT_SIZE_BYTES>(&input, &output);
        match result {
            Ok(_) => {
                trace!("Replied to echo")
            }
            Err(err) => {
                error!("Failed to reply to echo: {err:?}")
            }
        }

        ApexLinuxPartition::periodic_wait().unwrap();
    }
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
