use network_partition::prelude::*;
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
        let process_stack_size = self.config.stack_size.aperiodic_process;
        let max_mtu = self.get_max_mtu() as usize;
        let num_links = self.get_num_links();

        let vl_names = self.get_vl_names();

        let set_ports: Vec<TokenStream> = self.set_ports();

        let set_interfaces: Vec<TokenStream> = self.set_interfaces(&interface);
        let define_virtual_links: Vec<TokenStream> = self.define_virtual_links();
        let num_interfaces = self.get_num_interfaces();

        let interface_names = self.interface_names();

        let add_channels_to_links = self.add_channels_to_links();
        let add_interfaces_to_links = self.add_interfaces_to_links();

        let define_ports = self.define_ports();
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

            #( #define_ports )*

            #( #define_interfaces )*

            extern "C" fn entry_point() {
                info!("Running network partition");
                let mut interfaces: [&dyn Interface; #num_interfaces] = [ #( unsafe { #interface_names . get().unwrap() } ),* ];

                #( #define_virtual_links )*

                #( #add_channels_to_links )*

                #( #add_interfaces_to_links )*

                let mut links: [&dyn VirtualLink; #num_links] = [ #( &#vl_names ),* ];

                let mut scheduler = DeadlineRrScheduler::<#num_links>::new();
                let windows = [ #( #windows ),* ];
                for (vl_id, period) in windows {
                    _ = scheduler.insert(VirtualLinkId(vl_id), period);
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

                    trace!("Setting up ports");
                    #( #set_ports )*

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
            let mtu = i.mtu;
            let name = i.name.to_string();
            let var = name.to_uppercase();
            let var = format_ident!("IF_{var}");
            quote! { static mut #var: OnceCell<NetworkInterface<#mtu, #interface>> = OnceCell::new(); }
        })
        .collect()
    }

    fn set_ports(&self) -> Vec<TokenStream> {
        self.config
            .virtual_links
            .iter()
            .flat_map(|l| {
                l.ports.iter().map(|p| {
                    let mtu = l.msg_size;
                    let fifo_depth = l.fifo_depth;
                    match p {
                        Port::SamplingPortSource(p) => {
                            let channel = p.channel.as_str();
                            let name = p.channel.to_uppercase();
                            let var = format_ident!("PORT_{name}");
                            quote! {
                                let name = Name::from_str(#channel).unwrap();
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
                        Port::SamplingPortDestination(p) => {
                            let channel = p.channel.as_str();
                            let name = p.channel.to_uppercase();
                            let var = format_ident!("PORT_{name}");
                            let validity = p.validity.as_nanos();
                            quote! {
                                let name = Name::from_str(#channel).unwrap();
                                let validity = Duration::from_nanos(#validity as u64);
                                let src = match ctx.create_sampling_port_destination::<#mtu>(name, validity) {
                                    Ok(src) => src,
                                    Err(err) => {
                                        error!("{:?}", err);
                                        panic!("{:?}", err)
                                    }
                                };
                                unsafe { #var.set(src).unwrap(); }
                            }
                        }
                        Port::QueuingPortSender(p) => {
                            let channel = p.channel.as_str();
                            let name = p.channel.to_uppercase();
                            let var = format_ident!("PORT_{name}");
                            let fifo_depth = fifo_depth.unwrap_or(1);
                            quote! {
                                // TODO make configurable
                                let q_disc = QueuingDiscipline::FIFO;
                                let name = Name::from_str(#channel).unwrap();
                                let src = match ctx.create_queuing_port_sender::<#mtu, #fifo_depth>(name, q_disc, #fifo_depth) {
                                    Ok(src) => src,
                                    Err(err) => {
                                        error!("{:?}", err);
                                        panic!("{:?}", err)
                                    }
                                };
                                unsafe { #var.set(src).unwrap(); }
                            }
                        }
                        Port::QueuingPortReceiver(p) => {
                            let channel = p.channel.as_str();
                            let name = p.channel.to_uppercase();
                            let var = format_ident!("PORT_{name}");
                            let fifo_depth = fifo_depth.unwrap_or(1);
                            quote! {
                                // TODO make configurable
                                let q_disc = QueuingDiscipline::FIFO;
                                let name = Name::from_str(#channel).unwrap();
                                let src = match ctx.create_queuing_port_receiver::<#mtu, #fifo_depth>(name, q_disc, #fifo_depth) {
                                    Ok(src) => src,
                                    Err(err) => {
                                        error!("{:?}", err);
                                        panic!("{:?}", err)
                                    }
                                };
                                unsafe { #var.set(src).unwrap(); }
                            }
                        }
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
            let mtu = i.mtu;
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
        let mtu = vl.msg_size;
        let id = vl.id.into_inner();
        let sampling = vl.ports.iter().any(|p| p.sampling_port_destination().is_some() || p.sampling_port_source().is_some());
        let queuing = vl.ports.iter().any(|p| p.queuing_port_receiver().is_some() || p.queuing_port_sender().is_some());
        let name = format_ident!("vl_{id}");
        let interfaces = vl.interfaces.len();
        // TODO does this work or do we need ConstParam for this?
        if sampling {
            let sampling_port_sources = vl.ports.iter().filter(|p| p.sampling_port_source().is_some()).count();
            quote! {
                let mut #name = VirtualSamplingLink::<#mtu, #sampling_port_sources, #interfaces, Hypervisor>::new(VirtualLinkId::from(#id));
            }
        } else if queuing {
            let queuing_port_senders = vl.ports.iter().filter(|p| p.queuing_port_sender().is_some()).count();
            let depth = vl.fifo_depth.unwrap_or(1);
            quote! {
                let mut #name = VirtualQueuingLink::<#mtu, #depth, #queuing_port_senders, #interfaces, Hypervisor>::new(VirtualLinkId::from(#id));
            }
        } else {
            panic!("Virtual link without attached ports is invalid.")
        }
    }).collect()
    }

    fn add_channels_to_links(&self) -> Vec<TokenStream> {
        self.config
        .virtual_links
        .iter()
        .flat_map(|vl| {
            vl.ports.iter().filter_map(|port| {
                let id = vl.id;
                let vl_ident = format_ident!("vl_{id}");
                match port {
                    Port::SamplingPortSource(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        Some(quote!(#vl_ident.add_port_source(unsafe { #port_ident.get().unwrap().clone() });))
                    },
                    Port::SamplingPortDestination(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        Some(quote!(#vl_ident.add_port_destination(unsafe { #port_ident.get().unwrap() }.clone());))
                    },
                    Port::QueuingPortReceiver(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        Some(quote!(#vl_ident.add_port_destination(unsafe { #port_ident.get().unwrap() }.clone());))
                    },
                    Port::QueuingPortSender(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        Some(quote!(#vl_ident.add_port_source(unsafe { #port_ident.get().unwrap() }.clone());))
                    },
                }
            })
        })
        .collect()
    }

    fn define_ports(&self) -> Vec<TokenStream> {
        self.config.virtual_links.iter().flat_map(|vl| {
            vl.ports.iter().filter_map(|p| {
                match p {
                    Port::SamplingPortSource(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        Some(quote!(static mut #port_ident : OnceCell<SamplingPortSource<#msg_size, Hypervisor>> = OnceCell::new();))
                    },
                    Port::SamplingPortDestination(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        Some(quote!(static mut #port_ident : OnceCell<SamplingPortDestination<#msg_size, Hypervisor>> = OnceCell::new();))
                    },
                    Port::QueuingPortSender(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        let depth = vl.fifo_depth;
                        Some(quote!(static mut #port_ident : OnceCell<QueuingPortSender<#msg_size, #depth, Hypervisor>> = OnceCell::new();))
                    }
                    Port::QueuingPortReceiver(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        let depth = vl.fifo_depth;
                        Some(quote!(static mut #port_ident : OnceCell<QueuingPortReceiver<#msg_size, #depth, Hypervisor>> = OnceCell::new();))
                    }
                }
            })
        }).collect()
    }

    // TODO move getters to config module
    fn get_num_links(&self) -> usize {
        self.config.virtual_links.len()
    }

    fn get_max_mtu(&self) -> u32 {
        self.config
            .interfaces
            .iter()
            .map(|i| i.mtu)
            .max()
            .unwrap_or_default()
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
