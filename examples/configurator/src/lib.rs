//! This crate contains a stub mitigator that demonstrates how to provide the
//! router with a runtime configuration using a sampling port.

#![no_std]

pub use crate::run::*;

pub mod config {
    use a653rs_router::prelude::*;
    use core::time::Duration;

    pub(crate) fn config(cfg: ConfigOption) -> ConfigResult<2, 2> {
        match cfg {
            ConfigOption::EchoClient => Config::builder()
                .virtual_link(1, "EchoRequest")?
                .destination(1, "NodeB")?
                .schedule(1, Duration::from_millis(10))?
                .virtual_link(2, "NodeB")?
                .destination(2, "EchoReply")?
                .schedule(2, Duration::from_millis(10))?
                .build(),
            ConfigOption::EchoServer => Config::builder()
                .virtual_link(1, "NodeA")?
                .destination(1, "EchoRequest")?
                .schedule(1, Duration::from_millis(10))?
                .virtual_link(2, "EchoReply")?
                .destination(2, "NodeA")?
                .schedule(2, Duration::from_millis(10))?
                .build(),
            ConfigOption::EchoLocal => Config::builder()
                .virtual_link(1, "EchoRequestCl")?
                .destination(1, "EchoRequestSrv")?
                .schedule(1, Duration::from_millis(5))?
                .virtual_link(2, "EchoReplySrv")?
                .destination(2, "EchoReplyCl")?
                .schedule(2, Duration::from_millis(5))?
                .build(),
            ConfigOption::EchoLocalClientRemote => Config::builder()
                .virtual_link(1, "EchoRequestCl")?
                .destination(1, "NodeB")?
                .schedule(1, Duration::from_millis(10))?
                .virtual_link(2, "NodeB")?
                .destination(2, "EchoReplyCl")?
                .schedule(2, Duration::from_millis(10))?
                .build(),
            ConfigOption::Default => Ok(Config::default()),
        }
    }

    pub enum ConfigOption {
        EchoClient,
        EchoServer,
        EchoLocal,
        EchoLocalClientRemote,
        Default,
    }
}

mod run {
    use a653rs::{
        bindings::ApexSamplingPortP4,
        prelude::{ApexTimeP4Ext, SamplingPortSource},
    };
    use a653rs_router::prelude::Config;
    use log::*;

    use crate::config::{config, ConfigOption};

    pub fn configure<S: ApexSamplingPortP4 + ApexTimeP4Ext>(
        port: &SamplingPortSource<1000, S>,
    ) -> ! {
        // TODO load configs from parameter data item

        #[cfg(feature = "alt-local-client")]
        let configs = &[
            &config(ConfigOption::EchoLocalClientRemote).unwrap(),
            &config(ConfigOption::EchoLocal).unwrap(),
        ];

        #[cfg(feature = "client")]
        let configs = &[&config(ConfigOption::EchoClient).unwrap()];

        #[cfg(feature = "server")]
        let configs = &[&config(ConfigOption::EchoServer).unwrap()];

        #[cfg(feature = "local")]
        let configs = &[&config(ConfigOption::EchoLocal).unwrap()];

        #[cfg(not(any(
            feature = "alt-local-client",
            feature = "local",
            feature = "server",
            feature = "client"
        )))]
        let configs = &[&config(ConfigOption::Default).unwrap()];

        alternating(port, configs)
    }

    fn alternating<S: ApexSamplingPortP4 + ApexTimeP4Ext, const I: usize, const O: usize>(
        port: &SamplingPortSource<1000, S>,
        configs: &[&Config<I, O>],
    ) -> ! {
        const STEP: usize = 10usize;
        let mut counter = 0usize;
        let modulo = configs.len() * STEP;
        loop {
            let cfg = {
                counter += 1;
                counter %= modulo;
                configs[counter / STEP]
            };
            debug!("Sending configuration: {cfg:?}");
            let mut buf = [0u8; 1000];
            if let Ok(buf) = postcard::to_slice::<Config<I, O>>(cfg, &mut buf) {
                if let Err(e) = port.send(buf) {
                    error!("Failed to update config: {e:?}");
                }
            }
            <S as ApexTimeP4Ext>::periodic_wait().unwrap();
        }
    }
}
