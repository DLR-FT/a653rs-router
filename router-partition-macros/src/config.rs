use darling::{Error, FromMeta};
use itertools::Itertools;
use wrapped_types::{WrappedByteSize, WrappedDuration};

#[derive(Debug, Clone, FromMeta)]
#[darling(and_then = "Self::verify")]
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

impl StaticRouterConfig {
    fn verify(self) -> darling::Result<Self> {
        self.verify_no_duplicate_interfaces_names()?
            .verify_no_duplicate_port_names()
    }

    fn verify_no_duplicate_interfaces_names(self) -> darling::Result<Self> {
        let names: Vec<_> = self.interfaces.iter().map(|i| i.name.clone()).collect();
        self.verify_no_duplicate_names(&names)
    }

    fn verify_no_duplicate_port_names(self) -> darling::Result<Self> {
        let names: Vec<_> = self.ports.iter().map(|p| p.name().clone()).collect();
        self.verify_no_duplicate_names(&names)
    }

    fn verify_no_duplicate_names(self, names: &[String]) -> darling::Result<Self> {
        let mut acc = Error::accumulator();
        names.iter().duplicates().for_each(|name| {
            acc.push(Error::duplicate_field(&format!("Duplicate name {}", name)));
        });
        acc.finish()?;
        Ok(self)
    }
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

impl Port {
    fn name(&self) -> &String {
        match self {
            Port::SamplingIn { name, .. } => name,
            Port::SamplingOut { name, .. } => name,
            Port::QueuingIn { name, .. } => name,
            Port::QueuingOut { name, .. } => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_interface_names() {
        let cfg = syn::parse_str(
            r#"
            router_partition(
              hypervisor = foo::bar,
              interface(
                name = "Dup",
                kind = foo::bar,
                destination = "127.0.0.1:51234",
                mtu = "1KB",
                rate = "100MB",
                source = "127.0.0.1:54234"
              ),
              interface(
                name = "Dup",
                kind = foo::bar,
                destination = "127.0.0.1:51234",
                mtu = "1KB",
                rate = "100MB",
                source = "127.0.0.1:54234"
              ),
              inputs = 2, outputs = 2, mtu = "1.5KB",
              stack_size = "50MB",
              time_capacity = "5ms"
            )
            "#,
        )
        .unwrap();
        let cfg = StaticRouterConfig::from_meta(&cfg);
        assert!(cfg.is_err())
    }

    #[test]
    fn duplicate_port_names() {
        let cfg = syn::parse_str(
            r#"
            router_partition(
              hypervisor = foo::bar,
              port(queuing_in(name = "CAS", discipline = "FIFO", msg_size = "1KB", msg_count = "10")),
              port(queuing_out(name = "CAS", discipline = "FIFO", msg_size = "1KB", msg_count = "10")),
              inputs = 2, outputs = 2, mtu = "1.5KB",
              stack_size = "50MB",
              time_capacity = "5ms"
            )
            "#,
        )
        .unwrap();
        let cfg = StaticRouterConfig::from_meta(&cfg);
        assert!(cfg.is_err())
    }
}
