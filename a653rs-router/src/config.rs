use crate::{prelude::InterfaceConfig, types::VirtualLinkId};
use a653rs::bindings::{
    MessageRange, MessageSize, QueuingDiscipline as ApexQueuingDiscipline, StackSize,
};
use core::{ops::Deref, str::FromStr, time::Duration};
use heapless::{LinearMap, String, Vec};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const MAX_PORT_NAME: usize = 20;

/// The name of a hypervisor port.
/// Can have at-most 20 ASCII printable characters.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PortName(String<MAX_PORT_NAME>);

impl FromStr for PortName {
    type Err = RouterConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            String::from_str(s).map_err(|_| RouterConfigError::Port)?,
        ))
    }
}

impl Deref for PortName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl From<String<20>> for PortName {
    fn from(value: String<20>) -> Self {
        Self(value)
    }
}

/// The name of a network interface or socket address.
///
/// Some examples
/// - `10.10.0.1:81234`
/// - `TX`
/// - `eth0`
pub type InterfaceName = PortName;

/// Virtual link (VL) configuration data indexed by VL id.
pub type VirtualLinksConfig<const I: usize, const O: usize> =
    LinearMap<VirtualLinkId, VirtualLinkConfig<O>, I>;

/// Interface configuration data indexed by `InterfaceName`.
pub type InterfacesConfig<const IFS: usize> = LinearMap<InterfaceName, InterfaceConfig, IFS>;

/// Port configuration data indexed by `PortName`.
pub type PortsConfig<const PORTS: usize> = LinearMap<PortName, PortConfig, PORTS>;

/// Runtime configuration of the network partition.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RouterConfig<const IN: usize, const OUT: usize, const IFS: usize, const PORTS: usize> {
    /// Stack size limit
    pub stack_size: StackSize,

    /// Forwarding table
    #[cfg_attr(feature = "serde", serde(default))]
    pub virtual_links: VirtualLinksConfig<IN, OUT>,

    /// Interface configuration.
    /// The type of the interface depends on the platform.
    #[cfg_attr(feature = "serde", serde(default))]
    pub interfaces: InterfacesConfig<IFS>,

    /// Port configuration
    #[cfg_attr(feature = "serde", serde(default))]
    pub ports: PortsConfig<PORTS>,
}

/// Sampling port destination configuration
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingInCfg {
    /// Message size
    pub msg_size: MessageSize,
    /// Validity
    pub refresh_period: Duration,
}

/// Sampling port source configuration
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingOutCfg {
    /// Message size
    pub msg_size: MessageSize,
}

/// Queuing port discipline
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueuingDiscipline {
    /// FIFO
    Fifo,
    /// Priority queue
    Priority,
}

impl From<QueuingDiscipline> for ApexQueuingDiscipline {
    fn from(value: QueuingDiscipline) -> Self {
        match value {
            QueuingDiscipline::Fifo => ApexQueuingDiscipline::Fifo,
            QueuingDiscipline::Priority => ApexQueuingDiscipline::Priority,
        }
    }
}

/// Queuing port receiver configuration
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuingInCfg {
    /// Queuing descipline
    pub discipline: QueuingDiscipline,
    /// Maximum number of messages
    pub msg_count: MessageRange,
    /// Maximum message size
    pub msg_size: MessageSize,
}

/// Queuing port sender configuration
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuingOutCfg {
    /// Queuing discipline
    pub discipline: QueuingDiscipline,
    /// Maximum number of messages
    pub msg_count: MessageRange,
    /// Maximum message size
    pub msg_size: MessageSize,
}

/// Hypervisor port configuration
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PortConfig {
    /// Sampling port destination
    SamplingIn(SamplingInCfg),
    /// Sampling port source
    SamplingOut(SamplingOutCfg),
    /// Queuing port receiver
    QueuingIn(QueuingInCfg),
    /// Queuing port sender
    QueuingOut(QueuingOutCfg),
}

impl PortConfig {
    /// Queuing port receiver
    pub fn queuing_in(
        discipline: QueuingDiscipline,
        msg_count: MessageRange,
        msg_size: MessageSize,
    ) -> Self {
        Self::QueuingIn(QueuingInCfg {
            discipline,
            msg_count,
            msg_size,
        })
    }

    /// Queuing port sender
    pub fn queuing_out(
        discipline: QueuingDiscipline,
        msg_count: MessageRange,
        msg_size: MessageSize,
    ) -> Self {
        Self::QueuingOut(QueuingOutCfg {
            discipline,
            msg_count,
            msg_size,
        })
    }

    /// Sampling port destination
    pub fn sampling_in(msg_size: MessageSize, refresh_period: Duration) -> Self {
        Self::SamplingIn(SamplingInCfg {
            msg_size,
            refresh_period,
        })
    }

    /// Sampling port source
    pub fn sampling_out(msg_size: MessageSize) -> Self {
        Self::SamplingOut(SamplingOutCfg { msg_size })
    }
}

impl<const IN: usize, const OUT: usize, const IFS: usize, const PORTS: usize>
    RouterConfig<IN, OUT, IFS, PORTS>
{
    fn new(stack_size: usize) -> Self {
        Self {
            stack_size: stack_size as u32,
            virtual_links: Default::default(),
            interfaces: Default::default(),
            ports: Default::default(),
        }
    }

    /// Creates a new builder for a configuration.
    pub fn builder(stack_size: usize) -> RouterConfigBuilder<IN, OUT, IFS, PORTS> {
        sealed::greater_than_zero::<IN>();
        sealed::greater_than_zero::<OUT>();
        sealed::greater_than_zero::<IFS>();
        sealed::greater_than_zero::<PORTS>();
        RouterConfigBuilder::new(stack_size)
    }
}

/// Configuration error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterConfigError {
    /// The specified source of the virtual link is invalid.
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
    /// Insufficient storage for configuration
    Storage,
    /// Invalid configuration format
    Format,
}

/// Virtual link between one source and multiple destinations.
/// Sources and destinations can be on the network or local ports
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualLinkConfig<const D: usize> {
    /// Source
    #[cfg_attr(feature = "serde", serde(rename = "source"))]
    pub src: PortName,
    /// Destinations
    #[cfg_attr(feature = "serde", serde(rename = "destinations"))]
    pub dsts: Vec<PortName, D>,
    /// Minimum transmission interval
    pub period: Duration,
}

mod sealed {
    #[allow(dead_code)]
    #[allow(path_statements)]
    #[allow(unused_results)]
    #[allow(clippy::no_effect)]
    pub(crate) const fn power_of_two<const N: usize>() {
        Assert::<N, 0>::GREATER;
        Assert::<N, 0>::POWER_OF_TWO;
    }

    #[allow(dead_code)]
    #[allow(path_statements)]
    #[allow(unused_results)]
    #[allow(clippy::no_effect)]
    pub(crate) const fn greater_than_zero<const N: usize>() {
        Assert::<N, 0>::GREATER;
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
    pub aperiodic_process: u32,
}

/// Config builder
#[derive(Debug, Clone)]
pub struct RouterConfigBuilder<
    const IN: usize,
    const OUT: usize,
    const IFS: usize,
    const PORTS: usize,
> {
    cfg: RouterConfig<IN, OUT, IFS, PORTS>,
}

/// Result of applying a change to the configuration builder.
pub type BuilderResult<
    'a,
    const IN: usize,
    const OUT: usize,
    const IFS: usize,
    const PORTS: usize,
> = Result<&'a mut RouterConfigBuilder<IN, OUT, IFS, PORTS>, RouterConfigError>;

/// The result of building a configuration.
pub type CfgResult<const IN: usize, const OUT: usize, const IFS: usize, const PORTS: usize> =
    Result<RouterConfig<IN, OUT, IFS, PORTS>, RouterConfigError>;

impl<const IN: usize, const OUT: usize, const IFS: usize, const PORTS: usize>
    RouterConfigBuilder<IN, OUT, IFS, PORTS>
{
    /// Creates a new builder for a configuration with a given `stack_size`.
    pub fn new(stack_size: usize) -> Self {
        Self {
            cfg: RouterConfig::new(stack_size),
        }
    }

    /// Build the configuration.
    pub fn build(&self) -> CfgResult<IN, OUT, IFS, PORTS> {
        if self
            .cfg
            .virtual_links
            .iter()
            .any(|vl| vl.1.period.is_zero())
        {
            return Err(RouterConfigError::Schedule);
        }
        Ok(self.cfg.clone())
    }

    /// Adds a port to the configuration.
    ///
    /// # Errors
    /// Returns an error if the available storage is insufficient for storing
    /// the port configuration or the port name was invalid.
    pub fn port(
        &mut self,
        name: &str,
        port_cfg: PortConfig,
    ) -> BuilderResult<'_, IN, OUT, IFS, PORTS> {
        let name = PortName::from_str(name)?;
        self.cfg
            .ports
            .insert(name, port_cfg)
            .or(Err(RouterConfigError::Storage))?
            .map(|_| Err(RouterConfigError::Port))
            .unwrap_or(Ok(()))?;
        Ok(self)
    }

    /// Adds an interface to the configuration.
    ///
    /// # Errors
    /// Returns an error if the available storage is insufficient for storing
    /// the interface configuration or the name was invalid.
    pub fn interface(
        &mut self,
        name: &str,
        if_cfg: InterfaceConfig,
    ) -> BuilderResult<'_, IN, OUT, IFS, PORTS> {
        let name = InterfaceName::from_str(name).or(Err(RouterConfigError::Interface))?;
        self.cfg
            .interfaces
            .insert(name, if_cfg)
            .or(Err(RouterConfigError::Storage))?
            .map(|_| Err(RouterConfigError::Interface))
            .unwrap_or(Ok(()))?;
        Ok(self)
    }

    /// Adds a new destination to a virtual link.
    ///
    /// A port can be the name of an interface or the name of a hypervisor port.
    ///
    /// # Errors
    /// Returns an error if the destination does not exist in the configuration
    /// or the destination name was invalid.
    pub fn destination(
        &mut self,
        vl_id: u16,
        destination: &str,
    ) -> BuilderResult<'_, IN, OUT, IFS, PORTS> {
        let vl_id = VirtualLinkId::from(vl_id);
        let dst = PortName::from_str(destination).or(Err(RouterConfigError::Destination))?;
        if !self.contains_resource(&dst) {
            return Err(RouterConfigError::Destination);
        };
        let vl = self.find_vl(&vl_id)?;
        vl.dsts.push(dst).or(Err(RouterConfigError::Storage))?;
        Ok(self)
    }

    fn contains_resource(&mut self, dst: &PortName) -> bool {
        self.cfg.interfaces.contains_key(dst) || self.cfg.ports.contains_key(dst)
    }

    /// Adds a new slot to the schedule.
    ///
    /// `vl_id` refers to a virtual link that was created using
    /// `[virtual_link()]`
    ///
    /// # Errors
    /// Returns an error if the virtual link (VL) does not exist.
    pub fn schedule(
        &mut self,
        vl_id: u16,
        period: Duration,
    ) -> BuilderResult<'_, IN, OUT, IFS, PORTS> {
        let vl = VirtualLinkId::from(vl_id);
        let vl = self.find_vl(&vl)?;
        vl.period = period;
        Ok(self)
    }

    fn find_vl(
        &mut self,
        id: &VirtualLinkId,
    ) -> Result<&mut VirtualLinkConfig<OUT>, RouterConfigError> {
        self.cfg
            .virtual_links
            .get_mut(id)
            .ok_or(RouterConfigError::VirtualLink)
    }

    /// Adds a new virtual link.
    ///
    /// A port can be the name of an interface or the name of a hypervisor port.
    ///
    /// # Errors
    /// Returns an error if the source was invalid, a virtual link with this
    /// `vl_id` did already exist, or there is insufficient storage.
    pub fn virtual_link(
        &mut self,
        vl_id: u16,
        source: &str,
    ) -> BuilderResult<'_, IN, OUT, IFS, PORTS> {
        let src = PortName::from_str(source).or(Err(RouterConfigError::Source))?;
        if !self.contains_resource(&src) {
            return Err(RouterConfigError::Source);
        }
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
        let vl_added = self.cfg.virtual_links.insert(vl_id, vl).is_ok();
        if !vl_added {
            return Err(RouterConfigError::Storage);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::DataRate;

    use super::*;

    #[test]
    fn build_config() {
        _ = RouterConfig::<8, 8, 8, 8>::builder(10_000)
            .interface(
                "eth0",
                InterfaceConfig::new("NodeA", "NodeB", DataRate::b(10_000_000), 1_500),
            )
            .unwrap()
            .interface(
                "eth1",
                InterfaceConfig::new("NodeA", "NodeB", DataRate::b(10_000_000), 1_500),
            )
            .unwrap()
            .port(
                "Advisory_1",
                PortConfig::queuing_in(QueuingDiscipline::Fifo, 10, 1_0000),
            )
            .unwrap()
            .port(
                "Advisory_2",
                PortConfig::queuing_in(QueuingDiscipline::Fifo, 10, 1_0000),
            )
            .unwrap()
            .port(
                "FCC_1",
                PortConfig::queuing_in(QueuingDiscipline::Fifo, 10, 1_0000),
            )
            .unwrap()
            .port(
                "FCC_2",
                PortConfig::queuing_out(QueuingDiscipline::Fifo, 10, 1_0000),
            )
            .unwrap()
            .port(
                "FCC_3",
                PortConfig::queuing_out(QueuingDiscipline::Fifo, 10, 1_0000),
            )
            .unwrap()
            // VL 1
            .virtual_link(1, "Advisory_1")
            .unwrap()
            .destination(1, "eth0")
            .unwrap()
            .destination(1, "FCC_1")
            .unwrap()
            .schedule(1, Duration::from_millis(10))
            .unwrap()
            // VL2
            .virtual_link(2, "Advisory_2")
            .unwrap()
            .destination(2, "eth0")
            .unwrap()
            .destination(2, "FCC_2")
            .unwrap()
            .schedule(2, Duration::from_millis(20))
            .unwrap()
            // VL3
            .virtual_link(3, "eth0")
            .unwrap()
            .destination(3, "FCC_3")
            .unwrap()
            .destination(3, "eth1")
            .unwrap()
            .schedule(3, Duration::from_millis(40))
            .unwrap()
            .build()
            .unwrap();
    }
}
