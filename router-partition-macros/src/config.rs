use darling::FromMeta;
use wrapped_types::{WrappedByteSize, WrappedDuration};

#[derive(Debug, Clone, FromMeta)]
pub struct StaticRouterConfig {
    pub hypervisor: Hypervisor,
    pub inputs: usize,
    pub outputs: usize,
    pub mtu: WrappedByteSize,
    pub stack_size: WrappedByteSize,
    pub time_capacity: WrappedDuration,

    #[darling(multiple, rename = "interface")]
    pub interfaces: Vec<Interface>,

    #[darling(multiple, rename = "port")]
    pub ports: Vec<Port>,
}

type Hypervisor = syn::Path;
type InterfaceType = syn::Path;

#[derive(Debug, FromMeta, Clone)]
pub struct Interface {
    name: String,
    kind: InterfaceType,
    destination: String,
    mtu: WrappedByteSize,
    rate: WrappedByteSize,
    source: String,
}

impl Interface {
    pub fn into_inner(
        self,
    ) -> (
        String,
        InterfaceType,
        String,
        String,
        WrappedByteSize,
        WrappedByteSize,
    ) {
        (
            self.name,
            self.kind,
            self.source,
            self.destination,
            self.rate,
            self.mtu,
        )
    }
}

#[derive(Debug, FromMeta, Clone)]
pub enum Port {
    SamplingIn {
        name: String,
        msg_size: WrappedByteSize,
        refresh_period: WrappedDuration,
    },
    SamplingOut {
        name: String,
        msg_size: WrappedByteSize,
    },
    QueuingIn {
        name: String,
        discipline: String,
        msg_size: WrappedByteSize,
        msg_count: String,
    },
    QueuingOut {
        name: String,
        discipline: String,
        msg_size: WrappedByteSize,
        msg_count: String,
    },
}
