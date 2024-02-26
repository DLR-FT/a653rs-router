use a653rs::{
    bindings::{ApexProcessP4, ApexQueuingPortP4, ApexSamplingPortP4, StackSize},
    prelude::{Name, StartContext},
};

use crate::{
    config::VirtualLinksConfig,
    prelude::{
        CreateNetworkInterfaceId, Error, InterfacesConfig, PlatformNetworkInterface, PortsConfig,
    },
    process::{ProcessError, RouterProcess},
    router::{Router, RouterResources},
};

/// Router state.
///
/// This stores the structs by which the router prots, interfaces and processes
/// can be accessed.
#[derive(Debug)]
pub struct RouterState<H, P, const IFS: usize, const PORTS: usize>
where
    H: ApexProcessP4 + ApexQueuingPortP4 + ApexSamplingPortP4,
    P: PlatformNetworkInterface,
{
    resources: RouterResources<H, P, IFS, PORTS>,
    process: RouterProcess<H>,
}

/// An error occured while starting the router.
#[derive(Debug)]
pub enum PartitionError {
    Process(ProcessError),
}

impl From<ProcessError> for PartitionError {
    fn from(value: ProcessError) -> Self {
        Self::Process(value)
    }
}

impl<I, P, const IFS: usize, const PS: usize> RouterState<I, P, IFS, PS>
where
    I: ApexProcessP4 + ApexQueuingPortP4 + ApexSamplingPortP4,
    P: PlatformNetworkInterface,
{
    /// Initialize the router state and call the entry-point function of the
    /// router process.
    ///
    /// See also [RouterState::start].
    ///
    /// # Errors
    /// Returns an error describing what kind of resource failed to initialize.
    /// Enable the `log` feature for more debug information.
    pub fn create<C: CreateNetworkInterfaceId<P>>(
        ctx: &mut StartContext<I>,
        name: Name,
        interfaces: InterfacesConfig<IFS>,
        ports: PortsConfig<PS>,
        stack_size: StackSize,
        entry_point: extern "C" fn(),
    ) -> Result<Self, Error> {
        Ok(Self {
            resources: RouterResources::<I, P, IFS, PS>::create::<C>(interfaces, ports)?,
            process: RouterProcess::create(ctx, name, stack_size, entry_point)
                .map_err(Error::Process)?,
        })
    }

    /// Starts the router process.
    ///
    /// # Errors
    /// Returns an error wrapping the APEX error if starting the process fails
    /// for any reason.
    pub fn start(&self) -> Result<(), PartitionError> {
        self.process.start().map_err(PartitionError::from)
    }

    /// Call this from your entry-point function via a static variable. This is
    /// a current limitation of a653rs.
    pub fn router<const IN: usize, const OUT: usize, const BUF_LEN: usize>(
        &self,
        virtual_links_cfg: VirtualLinksConfig<IN, OUT>,
    ) -> Result<Router<IN, OUT>, Error> {
        Router::try_new(virtual_links_cfg, &self.resources)
    }
}
