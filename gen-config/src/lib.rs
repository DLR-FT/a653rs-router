use network_partition::prelude::*;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub struct ConfigGenerator<
    const VLS: usize,
    const PORTS: usize,
    const IFS: usize,
    const SCHEDULE_SLOTS: usize,
> {
    config: Config<VLS, PORTS, IFS, SCHEDULE_SLOTS>,
}

impl<const VLS: usize, const PORTS: usize, const IFS: usize, const SCHEDULE_SLOTS: usize>
    ConfigGenerator<VLS, PORTS, IFS, SCHEDULE_SLOTS>
{
    pub fn new(config: Config<VLS, PORTS, IFS, SCHEDULE_SLOTS>) -> Self {
        Self { config }
    }

    pub fn generate_network_partition(&self, hypervisor: TokenStream) -> TokenStream {
        self.config.validate().expect("Invalid config");
        let process_stack_size = self.config.stack_size.aperiodic_process;
        let max_mtu = self.get_max_mtu() as usize;
        let num_links = self.get_num_links();

        let vl_names = self.get_vl_names();

        let set_ports: Vec<TokenStream> = self.set_ports();

        let set_interfaces: Vec<TokenStream> = self.set_interfaces();
        let define_virtual_links: Vec<TokenStream> = self.define_virtual_links();
        let num_interfaces = self.get_num_interfaces();

        let interface_names = self.interface_names();

        let add_channels_to_links = self.add_channels_to_links();
        let add_interfaces_to_links = self.add_interfaces_to_links();

        let define_ports = self.define_ports();
        let define_interfaces = self.define_interfaces();

        let init_scheduler = self.init_scheduler();

        quote! {
            use apex_rs::prelude::*;
            use core::str::FromStr;
            use core::time::Duration;
            use core::result::Result::*;
            use log::{error, trace, info};
            use network_partition::prelude::*;
            use once_cell::unsync::OnceCell;
            use heapless::Vec;

            type Hypervisor = #hypervisor;

            #( #define_ports )*

            #( #define_interfaces )*

            extern "C" fn entry_point() {
                info!("Running network partition aperiodic process");

                let mut interfaces: [&dyn Interface; #num_interfaces] = [ #( unsafe { #interface_names . get().unwrap() } ),* ];

                #( #define_virtual_links )*

                #( #add_channels_to_links )*

                #( #add_interfaces_to_links )*

                let mut links: [&dyn VirtualLink; #num_links] = [ #( &#vl_names ),* ];

                #init_scheduler

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
                        name: Name::from_str("network_p").unwrap(),
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
                let vl_id = vl.id;
                let intf = self.config
                    .interfaces
                    .iter()
                    .find(|intf| {
                        let intf = match intf {
                            InterfaceConfig::Uart(intf) => intf.name.clone(),
                            InterfaceConfig::Udp(intf) => intf.name.clone(),
                        };
                        *i == intf
                    }).unwrap_or_else(|| panic!("Unable to find an interface with name {}", i.0.as_str()));
                let intf_id = match intf {
                    InterfaceConfig::Uart(intf) => intf.id.0,
                    InterfaceConfig::Udp(intf) => intf.id.0,
                };

                let vl_ident = format_ident!("vl_{vl_id}");
                quote! { #vl_ident.add_interface(NetworkInterfaceId::from(NetworkInterfaceId(#intf_id))); }
            })
        })
        .collect()
    }

    // TODO write functions for names of variables
    fn interface_names(&self) -> Vec<Ident> {
        self.config
            .interfaces
            .iter()
            .map(|i| {
                let id = match i {
                    InterfaceConfig::Udp(i) => i.name.clone().0.to_uppercase(),
                    InterfaceConfig::Uart(i) => i.name.clone().0.to_uppercase(),
                };
                format_ident!("IF_{id}")
            })
            .collect()
    }

    fn define_interfaces(&self) -> Vec<TokenStream> {
        self.config
        .interfaces
        .iter()
        .map(|i| match i {
            InterfaceConfig::Udp(i) => {
                let interface = quote! { network_partition_linux::UdpNetworkInterface };
                let name = i.name.clone().0.to_uppercase();
                let mtu = i.mtu;
                let var = format_ident!("IF_{name}");
                quote! {
                    use #interface;

                    static mut #var: OnceCell<NetworkInterface<#mtu, #interface>> = OnceCell::new();
                }
            }
            InterfaceConfig::Uart(i) => {
                let interface = quote! { network_partition_xng::UartNetworkInterface };
                let name = i.name.clone().0.to_uppercase();
                let mtu = i.mtu;
                let umtu = i.mtu as usize;
                let var = format_ident!("IF_{name}");
                quote! {
                    use #interface;

                    static mut #var: OnceCell<NetworkInterface<#mtu, #interface<#umtu>>> = OnceCell::new();
                }
            }
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

    fn set_interfaces(&self) -> Vec<TokenStream> {
        self.config
        .interfaces
        .iter()
        .map(|i| {
            match i {
                // TODO recreating the config struct should be obsolete when passing the config (for reconfiguration) directly to the partition.
                InterfaceConfig::Udp(i) => {
                    let name = i.name.to_string();
                    let var = i.name.to_string().to_uppercase();
                    let var = format_ident!("IF_{var}");
                    let destination = i.destination.to_string();
                    let rate = i.rate.0;
                    let mtu = i.mtu;
                    let id = i.id.0;
                    quote! {
                        let intf = UdpNetworkInterface::create_network_interface::<#mtu>(UdpInterfaceConfig { id: NetworkInterfaceId(#id), name: InterfaceName::from(#name), destination: #destination .into(), rate: DataRate::b(#rate), mtu: #mtu }).unwrap();
                        unsafe { #var.set(intf).unwrap(); }
                    }
                },
                InterfaceConfig::Uart(i) => {
                    let name = i.name.to_string();
                    let var = i.name.to_string().to_uppercase();
                    let var = format_ident!("IF_{var}");
                    let mtu = i.mtu;
                    let id = i.id.0;
                    quote! {
                        let intf = UartNetworkInterface::create_network_interface::<#mtu>(UartInterfaceConfig { id: NetworkInterfaceId(#id), name: InterfaceName::from(#name), mtu: #mtu }).unwrap();
                        unsafe { #var.set(intf).unwrap(); }
                    }
                }
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
            vl.ports.iter().map(|port| {
                let id = vl.id;
                let vl_ident = format_ident!("vl_{id}");
                match port {
                    Port::SamplingPortSource(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        quote!(#vl_ident.add_port_source(unsafe { #port_ident.get().unwrap().clone() });)
                    },
                    Port::SamplingPortDestination(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        quote!(#vl_ident.add_port_destination(unsafe { #port_ident.get().unwrap() }.clone());)
                    },
                    Port::QueuingPortReceiver(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        quote!(#vl_ident.add_port_destination(unsafe { #port_ident.get().unwrap() }.clone());)
                    },
                    Port::QueuingPortSender(port) => {
                        let port_id = port.channel.to_uppercase();
                        let port_ident = format_ident!("PORT_{port_id}");
                        quote!(#vl_ident.add_port_source(unsafe { #port_ident.get().unwrap() }.clone());)
                    },
                }
            })
        })
        .collect()
    }

    fn define_ports(&self) -> Vec<TokenStream> {
        self.config.virtual_links.iter().flat_map(|vl| {
            vl.ports.iter().map(|p| {
                match p {
                    Port::SamplingPortSource(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        quote!(static mut #port_ident : OnceCell<SamplingPortSource<#msg_size, Hypervisor>> = OnceCell::new();)
                    },
                    Port::SamplingPortDestination(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        quote!(static mut #port_ident : OnceCell<SamplingPortDestination<#msg_size, Hypervisor>> = OnceCell::new();)
                    },
                    Port::QueuingPortSender(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        let depth = vl.fifo_depth;
                        quote!(static mut #port_ident : OnceCell<QueuingPortSender<#msg_size, #depth, Hypervisor>> = OnceCell::new();)
                    }
                    Port::QueuingPortReceiver(port) => {
                        let channel = port.channel.to_uppercase();
                        let msg_size = vl.msg_size;
                        let port_ident = format_ident!("PORT_{channel}");
                        let depth = vl.fifo_depth;
                        quote!(static mut #port_ident : OnceCell<QueuingPortReceiver<#msg_size, #depth, Hypervisor>> = OnceCell::new();)
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
        let max_intf = self
            .config
            .interfaces
            .iter()
            .map(|i| match i {
                InterfaceConfig::Uart(i) => i.mtu,
                InterfaceConfig::Udp(i) => i.mtu,
            })
            .max()
            .unwrap_or_default();
        let max_ports = self
            .config
            .virtual_links
            .iter()
            .map(|l| l.msg_size)
            .max()
            .unwrap_or_default();
        std::cmp::max(max_intf, max_ports)
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

    fn init_scheduler(&self) -> TokenStream {
        let scheduler = self.config.schedule.clone();
        match scheduler {
            ScheduleConfig::DeadlineRr(cfg) => {
                let slots = cfg.slots.iter().map(|s| {
                    let vl = s.vl.0;
                    let period = s.period.as_nanos() as u64;
                    quote! { slots.push(DeadlineRrSlot { vl: VirtualLinkId(#vl), period: Duration::from_nanos(#period) }).unwrap(); }
                });
                quote! {
                    let mut slots = Vec::<DeadlineRrSlot, #SCHEDULE_SLOTS>::new();
                    #( #slots )*
                    let schedule = DeadlineRrScheduleConfig { slots };
                    let mut scheduler = DeadlineRrScheduler::<#SCHEDULE_SLOTS>::new(&schedule);
                }
            }
        }
    }
}
