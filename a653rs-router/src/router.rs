//! Router

use crate::{
    config::{
        InterfacesConfig, PortConfig, PortName, PortsConfig, RouterConfigError, VirtualLinksConfig,
    },
    error::Error,
    network::{CreateNetworkInterface, NetworkInterface, PayloadSize, PlatformNetworkInterface},
    ports::PortError,
    prelude::InterfaceName,
    scheduler::{DeadlineRrScheduler, ScheduleError, Scheduler, TimeSource},
    types::VirtualLinkId,
};

use a653rs::{
    bindings::{ApexQueuingPortP4, ApexSamplingPortP4},
    prelude::{
        QueuingPortReceiver, QueuingPortSender, SamplingPortDestination, SamplingPortSource,
        StartContext,
    },
};
use core::{fmt::Debug, marker::PhantomData, ops::Deref, str::FromStr, time::Duration};
use heapless::{FnvIndexMap, LinearMap, Vec};

#[derive(Debug)]
enum Port<H: ApexQueuingPortP4 + ApexSamplingPortP4> {
    SamplingIn(SamplingPortDestination<H>),
    SamplingOut(SamplingPortSource<H>),
    QueuingIn(QueuingPortReceiver<H>),
    QueuingOut(QueuingPortSender<H>),
}

/// Router resources
#[derive(Debug)]
pub struct RouterResources<H, P, const IFS: usize, const PORTS: usize>
where
    H: ApexQueuingPortP4 + ApexSamplingPortP4,
    P: PlatformNetworkInterface,
{
    _h: PhantomData<H>,
    _n: PhantomData<P>,
    ports: FnvIndexMap<PortName, Port<H>, PORTS>,
    net_ifs: FnvIndexMap<InterfaceName, NetworkInterface<P>, IFS>,
}

impl<H, P, const IFS: usize, const PORTS: usize> RouterResources<H, P, IFS, PORTS>
where
    H: ApexQueuingPortP4 + ApexSamplingPortP4,
    P: PlatformNetworkInterface,
{
    /// Creates the resources used by the router.
    ///
    /// This should be called during COLD_START and WARM_START.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is insufficient storage,
    /// there are ports of interfaces with duplicate names, the hypervisor
    /// failed to create a port, the network driver failed to create a network
    /// interface.
    pub fn create<C: CreateNetworkInterface<P>>(
        ctx: &mut StartContext<H>,
        interfaces_cfg: InterfacesConfig<IFS>,
        ports_cfg: PortsConfig<PORTS>,
    ) -> Result<Self, Error> {
        router_debug!(
            "Got interfaces {:?} and ports {:?}",
            interfaces_cfg,
            ports_cfg
        );
        let mut net_ifs: FnvIndexMap<PortName, NetworkInterface<P>, IFS> = Default::default();
        for (name, intf) in interfaces_cfg.iter() {
            let name = InterfaceName::from_str(name)?;
            let net_if = C::create_network_interface(intf)?;
            net_ifs
                .insert(name, net_if)
                .map_err(|_e| RouterConfigError::Storage)?
                .map(|_| Err(PortError::Create))
                .unwrap_or(Ok(()))?;
        }
        let mut ports: FnvIndexMap<PortName, Port<H>, PORTS> = Default::default();
        for (name, cfg) in ports_cfg.into_iter() {
            let name = PortName::from_str(name)?;
            let port = match cfg {
                PortConfig::SamplingIn(cfg) => Port::SamplingIn(
                    ctx.create_sampling_port_destination(
                        name.clone().try_into()?,
                        cfg.msg_size,
                        cfg.refresh_period,
                    )
                    .map_err(|_e| PortError::Create)?,
                ),
                PortConfig::SamplingOut(cfg) => Port::SamplingOut(
                    ctx.create_sampling_port_source(name.clone().try_into()?, cfg.msg_size)
                        .map_err(|_e| PortError::Create)?,
                ),
                PortConfig::QueuingIn(cfg) => Port::QueuingIn(
                    ctx.create_queuing_port_receiver(
                        name.clone().try_into()?,
                        cfg.msg_size,
                        cfg.msg_count,
                        cfg.discipline.clone().into(),
                    )
                    .map_err(|_e| PortError::Create)?,
                ),
                PortConfig::QueuingOut(cfg) => Port::QueuingOut(
                    ctx.create_queuing_port_sender(
                        name.clone().try_into()?,
                        cfg.msg_size,
                        cfg.msg_count,
                        cfg.discipline.clone().into(),
                    )
                    .map_err(|_e| PortError::Create)?,
                ),
            };
            ports
                .insert(name, port)
                .map_err(|_e| RouterConfigError::Storage)?
                .map(|_| Err(PortError::Create))
                .unwrap_or(Ok(()))?;
        }

        Ok(RouterResources {
            _h: Default::default(),
            _n: Default::default(),
            ports,
            net_ifs,
        })
    }
}

/// The router.
#[derive(Debug, Clone)]
pub struct Router<'a, const IN: usize, const OUT: usize> {
    routes: RouteTable<'a, IN, OUT>,
    scheduler: DeadlineRrScheduler<IN>,
}

impl<'a, const IN: usize, const OUT: usize> Router<'a, IN, OUT> {
    /// .
    /// Tries to initialize a new router from the given configuration.
    ///
    /// Creating the router from the given configuration and resources has no
    /// side-effects and may be attempted arbitrarily.
    ///
    /// # Errors
    /// This function will return an error if the configuration was invalid or
    /// did not match the provided resources.
    pub fn try_new<
        H: ApexQueuingPortP4 + ApexSamplingPortP4,
        P: PlatformNetworkInterface,
        const IFS: usize,
        const PORTS: usize,
    >(
        virtual_links_cfg: VirtualLinksConfig<IN, OUT>,
        resources: &'a RouterResources<H, P, IFS, PORTS>,
    ) -> Result<Self, Error> {
        let routes = RouteTable::<IN, OUT>::build(&virtual_links_cfg, resources)?;
        let scheduler_cfg: Vec<(VirtualLinkId, Duration), IN> = virtual_links_cfg
            .into_iter()
            .map(|(id, cfg)| (*id, cfg.period))
            .collect();
        let scheduler = DeadlineRrScheduler::try_new(&scheduler_cfg)?;
        let router = Self { routes, scheduler };
        Ok(router)
    }

    /// Forwards messages between the hypervisor and the network.
    pub fn forward<const B: usize, T: TimeSource>(
        &mut self,
        time_source: &T,
    ) -> Result<Option<VirtualLinkId>, Error> {
        let time = time_source.get_time().map_err(ScheduleError::from)?;
        if let Some(next) = self.scheduler.schedule_next(&time) {
            router_bench!(begin_virtual_link_scheduled, next.0 as u16);
            let res = self.routes.route::<B>(&next);
            router_bench!(end_virtual_link_scheduled, next.0 as u16);
            res?;
            Ok(Some(next))
        } else {
            Ok(None)
        }
    }
}

/// An input to a virtual link.
pub trait RouterInput {
    /// Receives a message and store it into `buf`.
    ///
    /// # Errors
    /// May return an error if receiving the message failed.
    fn receive<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], PortError>;

    /// Maximum transfer unit
    fn mtu(&self) -> PayloadSize;
}

/// An output from a virtual link.
pub trait RouterOutput {
    /// Sends `buf` to a virtual link on this `Output`.
    ///
    /// Returns a slice to the portion of `buf` that has *not* been transmitted.
    fn send(&self, buf: &[u8]) -> Result<(), PortError>;

    /// Maximum transfer unit
    fn mtu(&self) -> PayloadSize;
}

type FwdTable<'a, const I: usize, const O: usize> =
    LinearMap<VirtualLinkId, Vec<&'a dyn RouterOutput, O>, I>;

type Inputs<'a, const I: usize> = LinearMap<VirtualLinkId, &'a dyn RouterInput, I>;

/// The router containing the routing information.
#[derive(Default, Clone)]
pub struct RouteTable<'a, const I: usize, const O: usize> {
    inputs: Inputs<'a, I>,
    outputs: FwdTable<'a, I, O>,
}

impl<'a, const I: usize, const O: usize> RouteTable<'a, I, O> {
    /// Forwards a virtual link from its source to its destinations.
    fn route<const B: usize>(&self, vl: &VirtualLinkId) -> Result<(), Error> {
        let buf = &mut [0u8; B];
        let input = self.inputs.get(vl).ok_or(RouteError::InvalidVl)?;
        let buf = input.receive(buf)?;
        router_debug!("Received from {vl:?}: {buf:?}");
        let outs = self.outputs.get(vl).ok_or(RouteError::InvalidVl)?;
        for out in outs.into_iter() {
            out.send(buf).map_err(|e| {
                router_debug!("Failed to route {:?}", vl);
                e
            })?;
            router_debug!("Send to {vl:?}: {buf:?}");
        }
        Ok(())
    }

    fn build<H, P, const IFS: usize, const PORTS: usize>(
        virtual_links_cfg: &VirtualLinksConfig<I, O>,
        resources: &'a RouterResources<H, P, IFS, PORTS>,
    ) -> Result<RouteTable<'a, I, O>, RouterConfigError>
    where
        H: ApexQueuingPortP4 + ApexSamplingPortP4,
        P: PlatformNetworkInterface,
    {
        let mut inputs: LinearMap<PortName, &'a dyn RouterInput, I> = Default::default();
        let mut outputs: LinearMap<PortName, &'a dyn RouterOutput, O> = Default::default();
        for (name, net_if) in resources.net_ifs.iter() {
            inputs
                .insert(name.clone(), net_if)
                .or(Err(RouterConfigError::Storage))?
                .map(|_| Err(RouterConfigError::Interface))
                .unwrap_or(Ok(()))?;
            outputs
                .insert(name.clone(), net_if)
                .or(Err(RouterConfigError::Storage))?
                .map(|_| Err(RouterConfigError::Interface))
                .unwrap_or(Ok(()))?;
        }
        for (name, port) in resources.ports.iter() {
            let name = name.clone();
            match port {
                Port::SamplingIn(p) => inputs
                    .insert(name, p)
                    .or(Err(RouterConfigError::Storage))?
                    .map(|_| Err(RouterConfigError::Source))
                    .unwrap_or(Ok(()))?,
                Port::QueuingIn(p) => inputs
                    .insert(name, p)
                    .or(Err(RouterConfigError::Storage))?
                    .map(|_| Err(RouterConfigError::Source))
                    .unwrap_or(Ok(()))?,
                Port::SamplingOut(p) => outputs
                    .insert(name, p)
                    .or(Err(RouterConfigError::Storage))?
                    .map(|_| Err(RouterConfigError::Destination))
                    .unwrap_or(Ok(()))?,
                Port::QueuingOut(p) => outputs
                    .insert(name, p)
                    .or(Err(RouterConfigError::Storage))?
                    .map(|_| Err(RouterConfigError::Destination))
                    .unwrap_or(Ok(()))?,
            };
        }
        let mut b = &mut StateBuilder::default();
        for (v, cfg) in virtual_links_cfg.into_iter() {
            // Check for multiple uses of same source
            if virtual_links_cfg
                .iter()
                .filter(|(_, c)| c.src == cfg.src)
                .count()
                > 1
            {
                return Err(RouterConfigError::Source);
            }
            let inp = inputs.get(&cfg.src).ok_or_else(|| {
                router_debug!("Unknown input: {}", cfg.src.deref());
                RouterConfigError::Source
            })?;
            let outs: Result<Vec<_, O>, RouterConfigError> = cfg
                .dsts
                .iter()
                .map(|d| {
                    // Check for multiple uses of same destination
                    if virtual_links_cfg
                        .iter()
                        .flat_map(|(_, c)| c.dsts.iter())
                        .filter(|d_name| *d_name == d)
                        .count()
                        > 1
                    {
                        return Err(RouterConfigError::Destination);
                    }
                    outputs.get(d).ok_or_else(|| {
                        router_debug!("Unknown output {}", d.deref());
                        RouterConfigError::Destination
                    })
                })
                .map(|d| d.copied())
                .collect();
            let outs = outs?;
            b = b
                .route(v, *inp, &outs)
                .map_err(|_e| RouterConfigError::VirtualLink)?;
        }
        b.build()
    }
}

impl<'a, const I: usize, const O: usize> Debug for RouteTable<'a, I, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Router")
    }
}

type Routes<'a, const I: usize, const O: usize> =
    LinearMap<VirtualLinkId, (&'a dyn RouterInput, Vec<&'a dyn RouterOutput, O>), I>;

/// Builds a new router.
#[derive(Default)]
pub struct StateBuilder<'a, const I: usize, const O: usize> {
    vls: Routes<'a, I, O>,
}

impl<'a, const I: usize, const O: usize> Debug for StateBuilder<'a, I, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("RouterBuilder")
    }
}

impl<'a, const I: usize, const O: usize> StateBuilder<'a, I, O> {
    fn route(
        &mut self,
        vl: &VirtualLinkId,
        input: &'a dyn RouterInput,
        outputs: &Vec<&'a dyn RouterOutput, O>,
    ) -> Result<&mut Self, RouterConfigError> {
        if self.vls.contains_key(vl) {
            return Err(RouterConfigError::VirtualLink);
        }

        // Check if input and output message sizes match
        let input_msg_size = input.mtu();
        for outp in outputs.iter() {
            if outp.mtu() != input_msg_size {
                return Err(RouterConfigError::Destination);
            }
        }

        _ = self
            .vls
            .insert(*vl, (input, outputs.clone()))
            .map_err(|_e| RouterConfigError::Storage)?;
        Ok(self)
    }

    pub fn build(&self) -> Result<RouteTable<'a, I, O>, RouterConfigError> {
        let mut inputs = Inputs::default();
        let mut outputs = FwdTable::default();
        for (id, (i, o)) in self.vls.iter() {
            if inputs.contains_key(id) {
                return Err(RouterConfigError::Destination);
            }
            if outputs.contains_key(id) {
                return Err(RouterConfigError::Source);
            }
            _ = inputs.insert(*id, *i).or(Err(RouterConfigError::Storage));
            _ = outputs
                .insert(*id, o.clone())
                .or(Err(RouterConfigError::Storage));
        }
        Ok(RouteTable::<'a, I, O> { inputs, outputs })
    }
}

/// An error occured while routing a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteError {
    /// Invalid virtual link
    InvalidVl,
}
