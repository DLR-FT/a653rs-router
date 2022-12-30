use crate::config::Config;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub fn generate_network_partition(
    config: Config,
    hypervisor: TokenStream,
    interface: TokenStream,
) -> TokenStream {
    let process_stack_size = config.stack_size.periodic_process.as_u64() as u32;
    let max_mtu = get_max_mtu(&config) as usize;
    let num_links = get_num_links(&config);
    let min_interface_data_rate = get_min_interface_data_rate(&config);

    let vl_names = get_vl_names(&config);

    let vl_sampling_port_destinations: Vec<TokenStream> = set_sampling_port_destinations(&config);
    let vl_sampling_port_sources: Vec<TokenStream> = set_sampling_port_sources(&config);

    let set_interfaces: Vec<TokenStream> = set_interfaces(&config);
    let define_virtual_links: Vec<TokenStream> = define_virtual_links(&config);
    let num_interfaces = get_num_interfaces(&config);

    let interface_names = interface_names(&config);

    let add_sampling_port_sources_to_links = add_sampling_port_sources_to_links(&config);
    let add_sampling_port_destinations_to_links = add_sampling_port_destinations_to_links(&config);
    let add_queues_to_links = add_queues_to_links(&config);

    let define_port_sources = define_port_sources(&config);
    let define_port_destinations = define_port_destinations(&config);

    quote! {
        use apex_rs::prelude::*;
        use core::str::FromStr;
        use core::time::Duration;
        use log::{error, trace, LevelFilter};
        use network_partition::prelude::*;
        use once_cell::sync::OnceCell;
        use network_partition_linux::UdpInterface;

        type Hypervisor = #hypervisor;

        #( #define_port_sources )*

        #( #define_port_destinations )*

        #( static #interface_names : OnceCell<#interface<#max_mtu>> = OnceCell::new(); )*

        extern "C" fn entry_point() {
            let mut shaper = CreditBasedShaper::<#num_links>::new(DataRate::b(#min_interface_data_rate));
            let mut frame_buf = [0u8; #max_mtu];
            let mut interfaces: [&dyn Interface; #num_interfaces] = [ #( #interface_names . get().unwrap() ),* ];

            #( #define_virtual_links )*

            #( #add_sampling_port_sources_to_links )*

            #( #add_sampling_port_destinations_to_links )*

            #( #add_queues_to_links )*

            let mut links: [&mut dyn VirtualLink; #num_links] = [ #( &mut #vl_names ),* ];
            let mut forwarder = Forwarder::new(&mut frame_buf, &mut shaper, &mut links, &mut interfaces);

            loop {
                trace!("Continuing...");
                if let Err(err) = forwarder.forward::<Hypervisor>() {
                    error!("{err:?}");
                }
                trace!("Suspending...");
                Hypervisor::periodic_wait().unwrap();
           }
        }

        struct NetworkPartition;

        impl Partition<Hypervisor> for NetworkPartition {
            fn cold_start(&self, ctx: &mut StartContext<Hypervisor>) {

                let mut sockets = Hypervisor::receive_udp_sockets();

                #( #vl_sampling_port_destinations )*

                #( #vl_sampling_port_sources )*

                #( #set_interfaces )*

                ctx.create_process(ProcessAttribute {
                    period: SystemTime::Normal(Duration::ZERO),
                    time_capacity: SystemTime::Infinite,
                    entry_point,
                    stack_size: #process_stack_size,
                    base_priority: 1,
                    deadline: Deadline::Soft,
                    name: Name::from_str("network_partition").unwrap(),
                })
                .unwrap()
                .start()
                .unwrap()
            }

            fn warm_start(&self, ctx: &mut StartContext<Hypervisor>) {
                self.cold_start(ctx)
            }
        }
    }
}

// TODO write functions for names of variables

fn interface_names(config: &Config) -> Vec<Ident> {
    config
        .interfaces
        .iter()
        .map(|i| {
            let i = i.name.clone().0.to_uppercase();
            format_ident!("IF_{i}")
        })
        .collect()
}

fn set_sampling_port_destination(channel: &str, name: &str, mtu: u32, refresh: u64) -> TokenStream {
    let channel = channel.to_uppercase();
    let var = format_ident!("PORT_{channel}");
    quote! { #var.set(ctx.create_sampling_port_destination::<#mtu>(Name::from_str(#name).unwrap(), Duration::from_nanos(#refresh)).unwrap()).unwrap(); }
}

fn set_sampling_port_source(channel: &str, name: &str, mtu: u32) -> TokenStream {
    let channel = channel.to_uppercase();
    let var = format_ident!("PORT_{channel}");
    quote! { #var.set(ctx.create_sampling_port_source::<#mtu>(Name::from_str(#name).unwrap()).unwrap()).unwrap(); }
}
/// Generates TokenStreams that each initialize a sampling port.
/// The function generates all sampling port destinations for all virtual links.
/// The sampling port destinations must be named as static OnceCells.
/// The names of the sampling ports are VL<VL-ID>_<CHANNEL-NAME>.
fn set_sampling_port_destinations(config: &Config) -> Vec<TokenStream> {
    config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.ports.iter().filter_map(|p| {
                if let Some(p) = p.sampling_port_destination() {
                    Some(set_sampling_port_destination(
                        &p.channel,
                        &p.channel,
                        vl.msg_size.as_u64() as u32,
                        p.validity.as_nanos() as u64,
                    ))
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Generates TokenStreams that each initialize a sampling port source.
/// The function generates all sampling port sources for all virtual links.
/// The sampling port sources must be named as static OnceCells.
/// The names of the sampling ports are VL<VL-ID>_<CHANNEL-NAME>.
fn set_sampling_port_sources(config: &Config) -> Vec<TokenStream> {
    config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.ports.iter().filter_map(|p| {
                if let Some(p) = p.sampling_port_source() {
                    Some(set_sampling_port_source(
                        &p.channel,
                        &p.channel,
                        vl.msg_size.as_u64() as u32,
                    ))
                } else {
                    None
                }
            })
        })
        .collect()
}

fn set_interfaces(config: &Config) -> Vec<TokenStream> {
    let max_mtu = get_max_mtu(config) as usize;
    let min_interface_data_rate = get_min_interface_data_rate(config);
    config
        .interfaces
        .iter()
        .map(|i| {
            let name = i.name.clone().0.to_uppercase();
            let name = format_ident!("IF_{name}");
            // TODO make easily exchangeable for other interface types and multiple interfaces
            quote! {
                let sock = sockets.remove(0);
                sock.set_nonblocking(true).unwrap();
                sock.connect("127.0.0.1:34256").unwrap();
                let intf = UdpInterface::<#max_mtu>::new(sock, DataRate::b(#min_interface_data_rate));
                #name.set(intf).unwrap();
            }
        })
        .collect()
}

fn define_virtual_links(config: &Config) -> Vec<TokenStream> {
    config.virtual_links.iter().map(|vl| {
        let mtu = vl.msg_size.as_u64() as u32;
        let id = vl.id.into_inner();
        let ports = vl.ports.iter().filter(|p| p.sampling_port_source().is_some()).count();
        let name = format_ident!("vl_{id}");
        // TODO make dynamic based on config?
        let max_queue_len = 2usize;
        // TODO does this work or do we need ConstParam for this?
        quote! {
            let mut #name = VirtualLinkData::<#mtu, #ports, #max_queue_len, Hypervisor>::new(VirtualLinkId::from(#id));
        }
    }).collect()
}

fn add_sampling_port_sources_to_links(config: &Config) -> Vec<TokenStream> {
    config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.ports.iter().filter_map(|port| {
                let id = vl.id;
                let vl_ident = format_ident!("vl_{id}");
                if let Some(port) = port.sampling_port_source() {
                    let port_id = port.channel.to_uppercase();
                    let port_ident = format_ident!("PORT_{port_id}");
                    Some(quote!(#vl_ident.add_port_src(#port_ident.get().unwrap().clone());))
                } else {
                    None
                }
            })
        })
        .collect()
}

fn add_queues_to_links(config: &Config) -> Vec<TokenStream> {
    config
        .virtual_links
        .iter()
        .map(|vl| {
            let vl_id = vl.id.to_string();
            let vl_rate = vl.rate.as_u64();
            let vl_id = format_ident!("vl_{vl_id}");
            quote!(#vl_id = #vl_id.queue(&mut shaper, DataRate::b(#vl_rate));)
        })
        .collect()
}

fn add_sampling_port_destinations_to_links(config: &Config) -> Vec<TokenStream> {
    config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.ports.iter().filter_map(|port| {
                let id = vl.id;
                let vl_ident = format_ident!("vl_{id}");
                if let Some(port) = port.sampling_port_destination() {
                    let port_id = port.channel.to_uppercase();
                    let port_ident = format_ident!("PORT_{port_id}");
                    Some(quote!(#vl_ident.add_port_dst(#port_ident.get().unwrap().clone());))
                } else {
                    None
                }
            })
        })
        .collect()
}

fn define_port_sources(config: &Config) -> Vec<TokenStream> {
    config.virtual_links.iter().flat_map(|vl| {
        vl.ports.iter().filter_map(|p| {
        if let Some(port) = p.sampling_port_source() {
            let channel = port.channel.to_uppercase();
            let msg_size = vl.msg_size.as_u64() as u32;
            let port_ident = format_ident!("PORT_{channel}");
            // TODO pass in Hypervisor as parameter
            Some(
                quote!(static #port_ident : OnceCell<SamplingPortSource<#msg_size, Hypervisor>> = OnceCell::new();)
            )
        } else {
                None
            }
        })
    }).collect()
}

fn define_port_destinations(config: &Config) -> Vec<TokenStream> {
    config.virtual_links.iter().flat_map(|vl| {
        vl.ports.iter().filter_map(|p| {
        if let Some(port) = p.sampling_port_destination() {
            let channel = port.channel.to_uppercase();
            let msg_size = vl.msg_size.as_u64() as u32;
            let port_ident = format_ident!("PORT_{channel}");
            // TODO pass in Hypervisor as parameter
            Some(
                quote!(static #port_ident : OnceCell<SamplingPortDestination<#msg_size, Hypervisor>> = OnceCell::new();)
            )
        } else {
                None
            }
        })
    }).collect()
}

// TODO move getters to config module
fn get_num_links(config: &Config) -> usize {
    config.virtual_links.len()
}

fn get_min_interface_data_rate(config: &Config) -> u64 {
    config
        .interfaces
        .iter()
        .map(|i| i.rate)
        .min()
        .unwrap()
        .as_u64()
}

fn get_max_mtu(config: &Config) -> u64 {
    config
        .interfaces
        .iter()
        .map(|i| i.mtu)
        .max()
        .unwrap()
        .as_u64()
}

fn get_num_interfaces(config: &Config) -> usize {
    config.interfaces.len()
}

fn get_vl_names(config: &Config) -> Vec<Ident> {
    config
        .virtual_links
        .iter()
        .map(|vl| {
            let id = vl.id;
            format_ident!("vl_{id}")
        })
        .collect()
}
