use bytesize::ByteSize;
use core::include_str;
use serde::{Deserialize, Deserializer, Serialize};
use std::convert::From;
use std::time::Duration;

/// Configuration of the network partition
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ports: Vec<Port>,
    pub virtual_links: Vec<VirtualLinkConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigError;

impl From<serde_yaml::Error> for ConfigError {
    fn from(_: serde_yaml::Error) -> Self {
        // TODO use error somehow
        ConfigError {}
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VirtualLinkConfig {
    pub id: u16,
    #[serde(with = "humantime_serde")]
    pub period: Duration,
    #[serde(deserialize_with = "de_size_str")]
    pub msg_size: ByteSize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Port {
    SamplingPortSource(SamplingPortConfig),
    SamplingPortDestination(SamplingPortConfig),
    // Queuing(QueuingPort),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SamplingPortConfig {
    pub channel: String,
    #[serde(deserialize_with = "de_size_str")]
    pub msg_size: ByteSize,
    #[serde(with = "humantime_serde")]
    pub validity: Duration,
}

fn de_size_str<'de, D>(de: D) -> Result<ByteSize, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(de)?
        .parse::<ByteSize>()
        .map_err(serde::de::Error::custom)
}

impl Port {
    pub fn sampling_port_destination(&self) -> Option<SamplingPortConfig> {
        if let Self::SamplingPortDestination(q) = self {
            return Some(q.clone());
        }
        None
    }

    pub fn sampling_port_source(&self) -> Option<SamplingPortConfig> {
        if let Self::SamplingPortSource(q) = self {
            return Some(q.clone());
        }
        None
    }
}
