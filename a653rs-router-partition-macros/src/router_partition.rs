use crate::config::{Interface, Port, StaticRouterConfig};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::parse_quote;
use wrapped_types::{WrappedByteSize, WrappedDuration};

pub fn router_partition(name: syn::Ident, config: StaticRouterConfig) -> TokenStream {
    let router_mod: syn::Item = router_mod(&config).into();
    let partition_mod: syn::Item = partition_mod(&config).into();
    let module: syn::ItemMod = parse_quote! {
        pub mod #name {
            use a653rs::partition;
            use a653rs::prelude::PartitionExt;
            use a653rs_router::router_config;

            #router_mod

            #partition_mod

            pub fn run() {
                self::__router_partition::Partition.run();
            }
        }
    };
    quote! { #module }
}

struct Limits {
    inputs: usize,
    outputs: usize,
    mtu: WrappedByteSize,
}

impl Limits {
    fn new(inputs: usize, outputs: usize, mtu: WrappedByteSize) -> Self {
        Self {
            inputs,
            outputs,
            mtu,
        }
    }
}

impl From<Limits> for syn::ItemStruct {
    fn from(value: Limits) -> Self {
        let mtu = value.mtu.to_string();
        let inputs = Literal::usize_unsuffixed(value.inputs);
        let outputs = Literal::usize_unsuffixed(value.outputs);
        parse_quote! {
            #[limits(inputs = #inputs, outputs = #outputs, mtu = #mtu)]
            struct Limits;
        }
    }
}

struct IdedInterface<'a> {
    id: usize,
    interface: &'a Interface,
}

impl<'a> IdedInterface<'a> {
    fn new(id: usize, interface: &'a Interface) -> Self {
        Self { id, interface }
    }
}

impl<'a> From<IdedInterface<'a>> for syn::ItemStruct {
    fn from(value: IdedInterface) -> Self {
        let Interface {
            name,
            kind,
            rate,
            mtu,
            destination,
            source,
        } = value.interface.clone();
        let id = format_ident!("Interface{}", value.id);
        let rate = rate.to_string();
        let mtu = mtu.to_string();
        parse_quote! {
            #[interface(
                name = #name,
                interface_type = #kind,
                source = #source,
                destination = #destination,
                rate = #rate,
                mtu = #mtu
            )]
            struct #id;
        }
    }
}

fn router_mod(config: &StaticRouterConfig) -> syn::ItemMod {
    let limits = Limits::new(config.inputs, config.outputs, config.mtu.clone());
    let limits: syn::ItemStruct = limits.into();
    let interfaces = config
        .interfaces
        .iter()
        .enumerate()
        .map(|(id, interface)| syn::ItemStruct::from(IdedInterface::new(id, interface)));

    parse_quote! {
        #[router_config(a653rs_router::prelude::DeadlineRrScheduler)]
        pub(crate) mod __router_config {
            #limits
            #(#interfaces)*
        }
    }
}

struct IdedPort<'a> {
    id: usize,
    port: &'a Port,
}

impl<'a> IdedPort<'a> {
    fn new(id: usize, port: &'a Port) -> Self {
        Self { id, port }
    }

    fn ident(&self) -> syn::Ident {
        format_ident!("channel_{}", self.id)
    }

    fn in_port(&self) -> bool {
        matches!(self.port, Port::SamplingIn { .. } | Port::QueuingIn { .. })
    }

    fn name(&self) -> String {
        match &self.port {
            Port::SamplingIn { name, .. }
            | Port::SamplingOut { name, .. }
            | Port::QueuingIn { name, .. }
            | Port::QueuingOut { name, .. } => name.clone(),
        }
    }
}

impl<'a> From<&IdedPort<'a>> for syn::ItemStruct {
    fn from(value: &IdedPort) -> Self {
        let id = value.id;
        let id = format_ident!("Channel{}", id);
        let port = &value.port;
        let msg_size = port.msg_size().to_string();
        let name = port.name();
        match port {
            Port::SamplingIn { refresh_period, .. } => {
                let refresh_period = format!("{}ns", refresh_period.as_nanos());
                parse_quote! {
                    #[sampling_in(
                        name = #name,
                        msg_size = #msg_size,
                        refresh_period = #refresh_period
                    )]
                    struct #id;
                }
            }
            Port::SamplingOut { .. } => {
                parse_quote! {
                    #[sampling_out(
                        name = #name,
                        msg_size = #msg_size
                    )]
                    struct #id;
                }
            }
            Port::QueuingIn {
                discipline,
                msg_count,
                ..
            } => {
                let msg_count = msg_count.to_string();
                parse_quote! {
                    #[queuing_in(
                        name = #name,
                        discipline = #discipline,
                        msg_size = #msg_size,
                        msg_count = #msg_count
                    )]
                    struct #id;
                }
            }
            Port::QueuingOut {
                discipline,
                msg_count,
                ..
            } => {
                let msg_size = msg_size.to_string();
                let msg_count = msg_count.to_string();
                parse_quote! {
                    #[queuing_out(
                        name = #name,
                        discipline = #discipline,
                        msg_size = #msg_size,
                        msg_count = #msg_count
                    )]
                    struct #id;
                }
            }
        }
    }
}

fn partition_mod(config: &StaticRouterConfig) -> syn::ItemMod {
    let ided_ports: Vec<_> = config
        .ports
        .iter()
        .enumerate()
        .map(|(id, port)| IdedPort::new(id, port))
        .collect();

    let (in_ports, out_ports): (Vec<_>, Vec<_>) = ided_ports.iter().partition(|p| p.in_port());

    let (in_port_ids, in_port_names): (Vec<_>, Vec<_>) =
        in_ports.iter().map(|p| (p.ident(), p.name())).unzip();

    let (out_port_ids, out_port_names): (Vec<_>, Vec<_>) =
        out_ports.iter().map(|p| (p.ident(), p.name())).unzip();

    let warm_start = warm_start_fn(&in_port_ids, &out_port_ids);
    let cold_start = cold_start_fn();
    let hypervisor = hypervisor(config);

    let aperiodic_process = aperiodic_process_fn(
        &config.time_capacity,
        &config.stack_size,
        &hypervisor,
        &in_port_ids,
        &in_port_names,
        &out_port_ids,
        &out_port_names,
    );

    router_partition_mod(
        &hypervisor,
        &ided_ports,
        cold_start,
        warm_start,
        aperiodic_process,
    )
}

fn router_partition_mod(
    hypervisor: &syn::Path,
    ided_ports: &[IdedPort],
    cold_start: syn::ItemFn,
    warm_start: syn::ItemFn,
    aperiodic_process: syn::ItemFn,
) -> syn::ItemMod {
    let ports = ided_ports.iter().map(syn::ItemStruct::from);
    parse_quote! {
        #[partition(#hypervisor)]
        mod __router_partition {
            use a653rs_router::run_router;

            #[sampling_in(name = "RouterConfig", refresh_period = "10s", msg_size = "1KB")]
            struct RouterConfig;

            #( #ports )*

            #cold_start

            #warm_start

            #aperiodic_process
        }
    }
}

fn aperiodic_process_fn(
    time_capacity: &WrappedDuration,
    stack_size: &WrappedByteSize,
    hypervisor: &syn::Path,
    in_port_ids: &[syn::Ident],
    in_port_names: &[String],
    out_port_ids: &[syn::Ident],
    out_port_names: &[String],
) -> syn::ItemFn {
    let time_capacity = format!("{}ns", time_capacity.as_nanos());
    let stack_size = stack_size.to_string();
    let run_router = run_router_stmt(
        hypervisor,
        in_port_ids,
        in_port_names,
        out_port_ids,
        out_port_names,
    );

    parse_quote! {
        #[aperiodic(
            name = "router_process",
            time_capacity = #time_capacity,
            stack_size = #stack_size,
            base_priority = 5,
            deadline = "Soft"
        )]
        fn router_process(ctx: router_process::Context) {
            let router_config = ctx.router_config.expect("Failed to get router config port");
            #( let #in_port_ids = ctx.#in_port_ids.expect("Failed to get input"); )*
            #( let #out_port_ids = ctx.#out_port_ids.expect("Failed to get output"); )*
            #run_router
        }
    }
}

fn hypervisor(config: &StaticRouterConfig) -> syn::Path {
    config.hypervisor.clone()
}

fn cold_start_fn() -> syn::ItemFn {
    parse_quote! {
        #[start(cold)]
        fn cold_start(ctx: start::Context) {
            warm_start(ctx);
        }
    }
}

fn prefix_create(name: &syn::Ident) -> syn::Ident {
    format_ident!("create_{}", name)
}

fn warm_start_fn(in_port_ids: &[syn::Ident], out_port_ids: &[syn::Ident]) -> syn::ItemFn {
    let in_port_creates = in_port_ids.iter().map(prefix_create);
    let out_port_creates = out_port_ids.iter().map(prefix_create);
    parse_quote! {
        #[start(warm)]
        fn warm_start(mut ctx: start::Context) {
            ctx.create_router_config().expect("Failed to create router config port");
            #( ctx.#in_port_creates().expect("Failed to create input port {in_port_creates}"); )*
            #( ctx.#out_port_creates().expect("Failed to create output port {out_port_creates}"); )*
            ctx.create_router_process()
                .expect("Failed to create router process")
                .start()
                .expect("Failed to start router process");
        }
    }
}

fn run_router_stmt(
    hypervisor: &syn::Path,
    in_port_ids: &[syn::Ident],
    in_port_names: &[String],
    out_port_ids: &[syn::Ident],
    out_port_names: &[String],
) -> syn::Stmt {
    parse_quote! {
        run_router!(
            super::__router_config,
            #hypervisor {},
            router_config,
            [ #( (#in_port_names, #in_port_ids) ),* ],
            [ #( (#out_port_names, #out_port_ids) ),* ]
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::config::StaticRouterConfig;
    use darling::FromMeta;
    use quote::format_ident;

    #[test]
    fn empty_config() {
        let cfg = syn::parse_str(
            r#"
            router_partition(
              hypervisor = foo::bar,
              inputs = 2, outputs = 2, mtu = "1.5KB",
              stack_size = "50M",
              time_capacity = "5ms"
            )
            "#,
        )
        .unwrap();
        let cfg = StaticRouterConfig::from_meta(&cfg).unwrap();

        let name = format_ident!("{}", "my_router");
        let _code = crate::router_partition::router_partition(name, cfg);
    }

    #[test]
    fn interfaces_config() {
        let cfg = syn::parse_str(
            r#"
            router_partition(
              hypervisor = foo::bar,
              interface(
                name = "[51234-",
                kind = foo::bar,
                destination = "127.0.0.1:51234",
                mtu = "1KB",
                rate = "100MB",
                source = "127.0.0.1:54234"
              ),
              inputs = 2, outputs = 2, mtu = "1.5KB",
              port(queuing_in(name = "[[IGS]]", discipline = "FIFO", msg_size = "1KB", msg_count = "10")),
              port(queuing_out(name = "CAS", discipline = "FIFO", msg_size = "1KB", msg_count = "10")),
              stack_size = "50MB",
              time_capacity = "5ms"
            )
            "#,
        )
        .unwrap();
        let cfg = StaticRouterConfig::from_meta(&cfg).unwrap();
        let name = format_ident!("{}", "my_router");
        let _code = crate::router_partition::router_partition(name, cfg);
    }
}
