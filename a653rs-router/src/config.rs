use core::{fmt::Display, time::Duration};
use heapless::{LinearMap, String, Vec};

#[allow(unused_imports)]
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

#[cfg(all(feature = "std", feature = "serde"))]
use bytesize::ByteSize;

use crate::types::VirtualLinkId;

const MAX_PORT_NAME: usize = 20;
type Port = String<MAX_PORT_NAME>;

type FwdTable<const I: usize, const O: usize> = LinearMap<VirtualLinkId, VirtualLinkConfig<O>, I>;

/// Runtime configuration of the network partition.
///
/// ## Example
/// ```rust
/// use a653rs_router::prelude::*;
/// use core::time::Duration;
///
/// let cfg = || {
///     Config::<10, 8>::builder()
///         // VL 1
///         .virtual_link(1, "Advisory_1")?
///         .destination(1, "eth0")?
///         .destination(1, "FCC_1")?
///         .schedule(1, Duration::from_millis(10))?
///         // VL2
///         .virtual_link(2, "Advisory_2")?
///         .destination(2, "eth0")?
///         .destination(2, "FCC_2")?
///         .schedule(2, Duration::from_millis(20))?
///         // VL3
///         .virtual_link(3, "eth0")?
///         .destination(3, "FCC_3")?
///         .destination(3, "eth1")?
///         .schedule(3, Duration::from_millis(40))?
///         .build()
/// };
/// # assert!(cfg().is_ok())
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Config<const I: usize, const O: usize> {
    /// Forward table.
    #[cfg_attr(feature = "serde", serde(rename = "virtual_links"))]
    pub(crate) vls: FwdTable<I, O>,
}

impl<const I: usize, const O: usize> Config<I, O> {
    /// Creates a new builder for a configuration.
    pub fn builder() -> Builder<I, O> {
        sealed::greater_than_zero::<I>();
        sealed::greater_than_zero::<O>();
        Builder::default()
    }
}

/// Configuration error
#[derive(Debug, Clone)]
pub enum RouterConfigError {
    /// The specified source of the a virtual link is invalid.
    Source,
    /// The virtual links that are using an interface produce more traffic than
    /// can be serviced by the interface.
    DataRate,
    /// The specified port of a virtual link is invalid.
    Port,
    /// The specified virtual link is invalid either because it already exists
    /// or has an invalid format.
    VirtualLink,
    /// The specified network interface is invalid.
    Interface,
    /// The specified schedule or one of its slots is invalid.
    Schedule,
    /// The specified destination is illegal.
    Destination,
}

/// Virtual link between one source and multiple destinations.
/// Sources and destinations can be on the network or local ports
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct VirtualLinkConfig<const D: usize> {
    #[cfg_attr(feature = "serde", serde(rename = "source"))]
    pub src: Port,
    #[cfg_attr(feature = "serde", serde(rename = "destinations"))]
    pub dsts: Vec<Port, D>,
    #[cfg_attr(
        all(feature = "std", feature = "serde"),
        serde(with = "humantime_serde")
    )]
    pub period: Duration,
}

mod sealed {
    #[allow(dead_code)]
    #[allow(path_statements)]
    #[allow(unused_results)]
    #[allow(clippy::no_effect)]
    pub(crate) const fn power_of_two<const N: usize>() {
        Assert::<N, 0>::POWER_OF_TWO;
    }

    #[allow(dead_code)]
    #[allow(path_statements)]
    #[allow(unused_results)]
    #[allow(clippy::no_effect)]
    pub(crate) const fn greater_than_zero<const N: usize>() {
        Assert::<N, 1>::GREATER;
    }

    #[allow(dead_code)]
    pub struct Assert<const L: usize, const R: usize>;

    #[allow(dead_code)]
    impl<const L: usize, const R: usize> Assert<L, R> {
        pub const GREATER: usize = L - R - 1;
        pub const POWER_OF_TWO: usize = 0 - (L & (L - 1));
    }
}

/// Configures the amount of stack memory to reserve for the processes of the
/// partition.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct StackSizeConfig {
    /// The size of the memory to reserve on the stack for the aperiodic
    /// process.
    #[cfg_attr(
        all(feature = "serde", feature = "std"),
        serde(deserialize_with = "de_size_str_u32")
    )]
    pub aperiodic_process: u32,
}

#[cfg(all(feature = "std", feature = "serde"))]
fn de_size_str<'de, D>(de: D) -> Result<ByteSize, D::Error>
where
    D: Deserializer<'de>,
{
    std::string::String::deserialize(de)?
        .parse::<ByteSize>()
        .map_err(serde::de::Error::custom)
}

#[cfg(all(feature = "std", feature = "serde"))]
fn de_size_str_u32<'de, D>(de: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    de_size_str(de).map(|r| r.as_u64() as u32)
}

/// Config builder
#[derive(Default, Debug, Clone)]
pub struct Builder<const I: usize, const O: usize> {
    cfg: Config<I, O>,
}

/// Result of applying a change to the configuration builder.
pub type BuilderResult<'a, const I: usize, const O: usize> =
    Result<&'a mut Builder<I, O>, RouterConfigError>;

/// The result of building a configuration.
pub type ConfigResult<const I: usize, const O: usize> = Result<Config<I, O>, RouterConfigError>;

impl<const I: usize, const O: usize> Builder<I, O> {
    /// Build the configuration.
    pub fn build(&self) -> ConfigResult<I, O> {
        if self.cfg.vls.iter().any(|vl| vl.1.period.is_zero()) {
            return Err(RouterConfigError::Schedule);
        }
        Ok(self.cfg.clone())
    }

    /// Adds a new destination to a virtual link.
    ///
    /// A port can be the name of an interface or the name of a hypervisor port.
    pub fn destination(&mut self, vl_id: u16, destination: &str) -> BuilderResult<'_, I, O> {
        let vl_id = VirtualLinkId::from(vl_id);
        let dst = Port::try_from(destination).or(Err(RouterConfigError::Destination))?;
        let vl = self.find_vl(&vl_id)?;
        let destination_added = vl.dsts.push(dst);
        destination_added.or(Err(RouterConfigError::VirtualLink))?;
        Ok(self)
    }

    /// Adds a new slot to the schedule.
    pub fn schedule(&mut self, vl_id: u16, period: Duration) -> BuilderResult<'_, I, O> {
        let vl = VirtualLinkId::from(vl_id);
        let vl = self.find_vl(&vl)?;
        vl.period = period;
        Ok(self)
    }

    fn find_vl(
        &mut self,
        id: &VirtualLinkId,
    ) -> Result<&mut VirtualLinkConfig<O>, RouterConfigError> {
        self.cfg
            .vls
            .get_mut(id)
            .ok_or(RouterConfigError::VirtualLink)
    }

    /// Adds a new virtual link.
    ///
    /// A port can be the name of an interface or the name of a hypervisor port.
    pub fn virtual_link(&mut self, vl_id: u16, source: &str) -> BuilderResult<'_, I, O> {
        let src = Port::try_from(source).or(Err(RouterConfigError::Source))?;
        let vl_id = VirtualLinkId::from(vl_id);
        let duplicate = self.find_vl(&vl_id).is_ok();
        if duplicate {
            return Err(RouterConfigError::VirtualLink);
        }
        let vl = VirtualLinkConfig {
            src,
            dsts: Default::default(),
            period: Default::default(),
        };
        let vl_added = self.cfg.vls.insert(vl_id, vl).is_ok();
        if !vl_added {
            return Err(RouterConfigError::Source);
        }
        Ok(self)
    }
}

const MAX_INTERFACE_NAME: usize = 64;

/// The name of an interface. The name is platform-dependent.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct InterfaceName(pub String<MAX_INTERFACE_NAME>);

impl From<&str> for InterfaceName {
    #[inline]
    fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}

impl Display for InterfaceName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ConfigResult<10, 8> {
        Config::builder()
            // VL 1
            .virtual_link(1, "Advisory_1")?
            .destination(1, "eth0")?
            .destination(1, "FCC_1")?
            .schedule(1, Duration::from_millis(10))?
            // VL2
            .virtual_link(2, "Advisory_2")?
            .destination(2, "eth0")?
            .destination(2, "FCC_2")?
            .schedule(2, Duration::from_millis(20))?
            // VL3
            .virtual_link(3, "eth0")?
            .destination(3, "FCC_3")?
            .destination(3, "eth1")?
            .schedule(3, Duration::from_millis(40))?
            .build()
    }

    #[test]
    fn build_config() {
        assert!(config().is_ok())
    }

    #[cfg(all(feature = "std", feature = "serde"))]
    #[test]
    fn parse_config() {
        let cfg = r#"
            virtual_links:
              1:
                period: "10ms"
                source: "Advisory_1"
                destinations:
                  - "eth0"
                  - "FCC_1"
              2:
                period: "20ms"
                source: "Advisory_2"
                destinations:
                  - "eth0"
                  - "FCC_2"
              3:
                period: "40ms"
                source: "eth0"
                destinations:
                  - "FCC_3"
                  - "eth1"
        "#;
        let cfg = serde_yaml::from_str::<Config<10, 8>>(cfg);
        assert!(cfg.is_ok_and(|r| r.eq(&config().unwrap())))
    }
}
