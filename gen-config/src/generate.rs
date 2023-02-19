use network_partition::prelude::{Config, PayloadSize};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub struct ConfigGenerator<const VLS: usize, const PORTS: usize, const IFS: usize> {
    config: Config<VLS, PORTS, IFS>,
}

impl<const VLS: usize, const PORTS: usize, const IFS: usize> ConfigGenerator<VLS, PORTS, IFS> {
    pub fn new(config: Config<VLS, PORTS, IFS>) -> Self {
        Self { config }
    }

    pub fn generate_network_partition(
        &self,
        hypervisor: TokenStream,
        interface: TokenStream,
    ) -> TokenStream {
        let process_stack_size = self.config.stack_size.periodic_process.as_u64() as u32;
        let max_mtu = self.get_max_mtu() as usize;
        let num_links = self.get_num_links();

        let vl_names = self.get_vl_names();

        let vl_sampling_port_destinations: Vec<TokenStream> = self.set_sampling_port_destinations();
        let vl_sampling_port_sources: Vec<TokenStream> = self.set_sampling_port_sources();

        let set_interfaces: Vec<TokenStream> = self.set_interfaces(&interface);
        let define_virtual_links: Vec<TokenStream> = self.define_virtual_links();
        let num_interfaces = self.get_num_interfaces();

        let interface_names = self.interface_names();

        let add_sampling_port_sources_to_links = self.add_sampling_port_sources_to_links();
        let add_sampling_port_destinations_to_links =
            self.add_sampling_port_destinations_to_links();
        let add_interfaces_to_links = self.add_interfaces_to_links();

        let define_port_sources = self.define_port_sources();
        let define_port_destinations = self.define_port_destinations();
        let define_interfaces = self.define_interfaces(&interface);
        let windows = self.define_windows();

        quote! {
            use apex_rs::prelude::*;
            use core::str::FromStr;
            use core::time::Duration;
            use core::result::Result::*;
            use log::{error, trace, info};
            use network_partition::prelude::*;
            use once_cell::unsync::OnceCell;

            type Hypervisor = #hypervisor;

            #( #define_port_sources )*

            #( #define_port_destinations )*

            #( #define_interfaces )*

            extern "C" fn entry_point() {
                info!("Running network partition");
                let mut interfaces: [&dyn Interface; #num_interfaces] = [ #( unsafe { #interface_names . get().unwrap() } ),* ];

                #( #define_virtual_links )*

                #( #add_sampling_port_sources_to_links )*

                #( #add_sampling_port_destinations_to_links )*

                #( #add_interfaces_to_links )*

                let mut links: [&dyn VirtualLink; #num_links] = [ #( &#vl_names ),* ];

                let mut scheduler = DeadlineRrScheduler::<#num_links>::new();
                let windows = [ #( #windows ),* ];
                for (vl_id, period) in windows {
                    scheduler.insert(VirtualLinkId(vl_id), period);
                }

                let mut forwarder = Forwarder::new(&mut scheduler, &mut links, &mut interfaces);
                loop {
                    let mut frame_buf = [0u8; #max_mtu];
                    forwarder.forward::<Hypervisor>(&mut frame_buf);
                }
            }

            struct NetworkPartition;

            impl Partition<Hypervisor> for NetworkPartition {
                fn cold_start(&self, ctx: &mut StartContext<Hypervisor>) {
                    trace!("Cold start");

                    trace!("Setting up sampling port destinations");
                    #( #vl_sampling_port_destinations )*

                    trace!("Setting up sampling port sources");
                    #( #vl_sampling_port_sources )*

                    trace!("Setting up interfaces");
                    #( #set_interfaces )*

                    trace!("Starting process");

                    let process = match ctx.create_process(ProcessAttribute {
                        period: SystemTime::Infinite,
                        time_capacity: SystemTime::Infinite,
                        entry_point,
                        stack_size: #process_stack_size,
                        base_priority: 5,
                        deadline: Deadline::Soft,
                        name: Name::from_str("np").unwrap(),
                    }) {
                        Ok(process) => process,
                        Err(err) => {
                            panic!("Failed to create process: {:?}", err);
                        }
                    };
                    process.start().unwrap()
                }

                fn warm_start(&self, ctx: &mut StartContext<Hypervisor>) {
                    self.cold_start(ctx)
                }
            }
        }
    }

    fn add_interfaces_to_links(&self) -> Vec<TokenStream> {
        self.config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.interfaces.iter().map(|i| {
                let id = vl.id;
                let vl_ident = format_ident!("vl_{id}");
                let if_id = self.config
                    .interfaces
                    .iter()
                    .find(|intf| *i.0 == intf.name.0)
                    .unwrap()
                    .id.0;
                quote! { #vl_ident.add_interface(NetworkInterfaceId::from(NetworkInterfaceId(#if_id))); }
            })
        })
        .collect()
    }

    fn define_windows(&self) -> Vec<TokenStream> {
        self.config
            .virtual_links
            .iter()
            .map(|l| {
                let id = l.id.0;
                let rate = l.rate.as_nanos() as u64;
                quote! { (#id, Duration::from_nanos(#rate)) }
            })
            .collect()
    }

    // TODO write functions for names of variables

    fn interface_names(&self) -> Vec<Ident> {
        self.config
            .interfaces
            .iter()
            .map(|i| {
                let i = i.name.clone().0.to_uppercase();
                format_ident!("IF_{i}")
            })
            .collect()
    }

    fn define_interfaces(&self, interface: &TokenStream) -> Vec<TokenStream> {
        self.config
        .interfaces
        .iter()
        .map(|i| {
            let mtu = i.mtu.as_u64() as PayloadSize;
            let name = i.name.to_string();
            let var = name.to_uppercase();
            let var = format_ident!("IF_{var}");
            quote! { static mut #var: OnceCell<NetworkInterface<#mtu, #interface>> = OnceCell::new(); }
        })
        .collect()
    }

    fn set_sampling_port_destination(
        channel: &str,
        name: &str,
        mtu: u32,
        refresh: u64,
    ) -> TokenStream {
        let channel = channel.to_uppercase();
        let var = format_ident!("PORT_{channel}");
        quote! { unsafe { #var.set(ctx.create_sampling_port_destination::<#mtu>(Name::from_str(#name).unwrap(), Duration::from_nanos(#refresh)).unwrap()).unwrap(); } }
    }

    fn set_sampling_port_source(channel: &str, name: &str, mtu: u32) -> TokenStream {
        let channel = channel.to_uppercase();
        let var = format_ident!("PORT_{channel}");
        quote! {
            let name = Name::from_str(#name).unwrap();
            let src = match ctx.create_sampling_port_source::<#mtu>(name) {
                Ok(src) => src,
                Err(err) => {
                    error!("{:?}", err);
                    panic!("{:?}", err)
                }
            };
            unsafe { #var.set(src).unwrap(); }
        }
    }

    /// Generates TokenStreams that each initialize a sampling port.
    /// The function generates all sampling port destinations for all virtual links.
    /// The sampling port destinations must be named as static OnceCells.
    /// The names of the sampling ports are VL<VL-ID>_<CHANNEL-NAME>.
    fn set_sampling_port_destinations(&self) -> Vec<TokenStream> {
        self.config
            .virtual_links
            .iter()
            .flat_map(|vl| {
                vl.ports.iter().filter_map(|p| {
                    if let Some(p) = p.sampling_port_destination() {
                        Some(Self::set_sampling_port_destination(
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
    fn set_sampling_port_sources(&self) -> Vec<TokenStream> {
        self.config
            .virtual_links
            .iter()
            .flat_map(|vl| {
                vl.ports.iter().filter_map(|p| {
                    if let Some(p) = p.sampling_port_source() {
                        Some(Self::set_sampling_port_source(
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

    fn set_interfaces(&self, nil: &TokenStream) -> Vec<TokenStream> {
        self.config
        .interfaces
        .iter()
        .map(|i| {
            let name = i.name.to_string();
            let var = i.name.to_string().to_uppercase();
            let var = format_ident!("IF_{var}");
            let mtu = i.mtu.as_u64() as PayloadSize;
            let destination = i.destination.to_string();
            let rate = i.rate.as_u64();
            quote! {
                let intf = #nil::create_network_interface::<#mtu>(#name, #destination, DataRate::b(#rate)).unwrap();
                unsafe { #var.set(intf).unwrap(); }
            }
        })
        .collect()
    }

    fn define_virtual_links(&self) -> Vec<TokenStream> {
        self.config.virtual_links.iter().map(|vl| {
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

    fn add_sampling_port_sources_to_links(&self) -> Vec<TokenStream> {
        self.config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.ports.iter().filter_map(|port| {
                let id = vl.id;
                let vl_ident = format_ident!("vl_{id}");
                if let Some(port) = port.sampling_port_source() {
                    let port_id = port.channel.to_uppercase();
                    let port_ident = format_ident!("PORT_{port_id}");
                    Some(quote!(#vl_ident.add_port_src(unsafe { #port_ident.get().unwrap().clone() });))
                } else {
                    None
                }
            })
        })
        .collect()
    }

    fn add_sampling_port_destinations_to_links(&self) -> Vec<TokenStream> {
        self.config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.ports.iter().filter_map(|port| {
                let id = vl.id;
                let vl_ident = format_ident!("vl_{id}");
                if let Some(port) = port.sampling_port_destination() {
                    let port_id = port.channel.to_uppercase();
                    let port_ident = format_ident!("PORT_{port_id}");
                    Some(quote!(#vl_ident.add_port_dst(unsafe { #port_ident.get().unwrap() }.clone());))
                } else {
                    None
                }
            })
        })
        .collect()
    }

    fn define_port_sources(&self) -> Vec<TokenStream> {
        self.config.virtual_links.iter().flat_map(|vl| {
        vl.ports.iter().filter_map(|p| {
        if let Some(port) = p.sampling_port_source() {
            let channel = port.channel.to_uppercase();
            let msg_size = vl.msg_size.as_u64() as u32;
            let port_ident = format_ident!("PORT_{channel}");
            // TODO pass in Hypervisor as parameter
            Some(
                quote!(static mut #port_ident : OnceCell<SamplingPortSource<#msg_size, Hypervisor>> = OnceCell::new();)
            )
        } else {
                None
            }
        })
        }).collect()
    }

    fn define_port_destinations(&self) -> Vec<TokenStream> {
        self.config.virtual_links.iter().flat_map(|vl| {
        vl.ports.iter().filter_map(|p| {
        if let Some(port) = p.sampling_port_destination() {
            let channel = port.channel.to_uppercase();
            let msg_size = vl.msg_size.as_u64() as u32;
            let port_ident = format_ident!("PORT_{channel}");
            // TODO pass in Hypervisor as parameter
            Some(
                quote!(static mut #port_ident : OnceCell<SamplingPortDestination<#msg_size, Hypervisor>> = OnceCell::new();)
            )
        } else {
                None
            }
        })
    }).collect()
    }

    // TODO move getters to config module
    fn get_num_links(&self) -> usize {
        self.config.virtual_links.len()
    }

    fn get_max_mtu(&self) -> u64 {
        self.config
            .interfaces
            .iter()
            .map(|i| i.mtu)
            .max()
            .unwrap_or_default()
            .as_u64()
    }

    fn get_num_interfaces(&self) -> usize {
        self.config.interfaces.len()
    }

    fn get_vl_names(&self) -> Vec<Ident> {
        self.config
            .virtual_links
            .iter()
            .map(|vl| {
                let id = vl.id;
                format_ident!("vl_{id}")
            })
            .collect()
    }
}
