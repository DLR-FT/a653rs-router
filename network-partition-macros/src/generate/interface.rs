use quote::format_ident;
use syn::{parse_quote, ItemMod};

use crate::parse::interface::Interface;

use super::GenMod;

impl GenMod for Interface {
    fn gen_mod(&self) -> syn::Result<ItemMod> {
        let name = &self.name;
        let ident = format_ident!("{}", self.id);
        let interface_type = &self.interface_type.path;
        let mtu = self.mtu.bytes() as usize;
        let rate = self.rate.bytes();
        let source = self.source.as_str();
        let destination = self.destination.as_str();
        Ok(parse_quote! {
            mod #ident {
                use ::a653rs_router::prelude::*;
                use ::a653rs_router::error::*;

                pub fn init() -> Result<(), Error> {
                    let intf = #interface_type :: create_network_interface::< #mtu >( InterfaceConfig::new( #source, #destination, DataRate::b( #rate ), #mtu ) )?;
                    unsafe {
                        VALUE.replace(intf);
                    }
                    Ok(())
                }

                pub const NAME: &str = #name;

                pub static mut VALUE: Option< NetworkInterface < #mtu, #interface_type< #mtu > >> = None;
            }
        })
    }
}
