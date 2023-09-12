use crate::generate::GenMod;
use crate::parse::{interface::Interface, router::Router};
use darling::ToTokens;
use syn::spanned::Spanned;
use syn::{parse_quote, token::Brace, Item, ItemMod, ItemType};
use syn::{Ident, ItemConst};

use super::GenerateStream;

impl GenerateStream for Router {
    fn gen_stream(&self, input: &mut syn::ItemMod) -> syn::Result<proc_macro2::TokenStream> {
        let span = input.span();
        let (_brace, content): &mut (Brace, Vec<Item>) = input
            .content
            .as_mut()
            .ok_or_else(|| syn::Error::new(span, "Missing content"))?;

        // Type alias for scheduler.
        content.push(self.gen_sched_type_alias().into());

        // Number of buffer length, inputs and outputs
        content.push(self.gen_inputs_const().into());
        content.push(self.gen_outputs_const().into());
        content.push(self.gen_total_inputs_const().into());
        content.push(self.gen_total_outputs_const().into());
        content.push(self.gen_buffer_len_const().into());

        // Generates module with GetResources implementation for getting all interfaces
        // by name.
        content.extend(self.gen_interface_mods()?.map(Into::into));
        // Generates start method with scheduler + router + reconfigure + forward loop
        content.push(self.gen_start().into());

        let ts = input.to_token_stream();

        Ok(ts)
    }
}

impl Router {
    fn gen_sched_type_alias(&self) -> ItemType {
        let sched = &self.scheduler;
        let slots = self.inputs + self.interfaces.len();
        parse_quote!(
            type AScheduler = #sched<#slots>;
        )
    }

    fn gen_interface_mods(&self) -> syn::Result<impl Iterator<Item = ItemMod>> {
        let mut intfmods = self
            .interfaces
            .iter()
            .map(Interface::gen_mod)
            .collect::<syn::Result<Vec<ItemMod>>>()?;
        intfmods.push(self.gen_interface_resources(&intfmods));
        Ok(intfmods.into_iter())
    }

    fn gen_start(&self) -> ItemMod {
        parse_quote! {
            pub mod start {
                use super::*;
                use ::network_partition::prelude::*;
                use ::network_partition::error::*;

                pub fn run(
                    time: &'_ dyn TimeSource,
                    router_config: &'_ dyn RouterInput,
                    inputs: [(&str, &'_ dyn RouterInput); INPUTS],
                    outputs: [(&str, &'_ dyn RouterOutput); OUTPUTS]
                ) -> ! {
                    super::interfaces::init().expect("Failed to initialize interfaces");
                    let mut resources = Resources::<TOTAL_INPUTS, TOTAL_OUTPUTS>::new();
                    inputs.into_iter().for_each(|(n,v)| resources.insert_input(n, v).unwrap());
                    outputs.into_iter().for_each(|(n,v)| resources.insert_output(n, v).unwrap());
                    super::interfaces::inputs().into_iter().for_each(|(n,v)| resources.insert_input(n, v).unwrap());
                    super::interfaces::outputs().into_iter().for_each(|(n,v)| resources.insert_output(n, v).unwrap());
                    let mut scheduler = AScheduler::default();
                    network_partition::run::<TOTAL_INPUTS, TOTAL_OUTPUTS, BUF_LEN>(time, router_config, resources, &mut scheduler)
                }
            }
        }
    }

    fn gen_interface_resources(&self, intfmods: &[ItemMod]) -> ItemMod {
        let interfaces: Vec<Ident> = intfmods.iter().map(|i| i.ident.clone()).collect();
        let io = interfaces.len();
        parse_quote! {
            mod interfaces {
                use ::network_partition::prelude::*;
                use ::network_partition::error::*;

                use super::*;

                pub fn inputs<'a>() ->[(&'static str, &'a dyn RouterInput); #io ] {
                    [ #( ( #interfaces ::NAME , unsafe { #interfaces ::VALUE.as_ref().expect("Interface not initialized") } ) ),* ]
                }

                pub fn outputs<'a>() -> [(&'static str, &'a dyn RouterOutput); #io ] {
                    [ #( ( #interfaces ::NAME , unsafe { #interfaces ::VALUE.as_ref().expect("Interface not initialized") } ) ),* ]
                }

                pub fn init() -> Result<(), Error> {
                    #( #interfaces ::init() ?; )*
                    Ok(())
                }
            }
        }
    }

    fn gen_inputs_const(&self) -> ItemConst {
        let inputs = &self.inputs;
        parse_quote!(const INPUTS: usize = #inputs;)
    }

    fn gen_outputs_const(&self) -> ItemConst {
        let outputs = &self.outputs;
        parse_quote!(const OUTPUTS: usize = #outputs;)
    }

    fn gen_total_inputs_const(&self) -> ItemConst {
        let inputs = &(self.inputs + self.interfaces.len());
        parse_quote!(const TOTAL_INPUTS: usize = #inputs;)
    }

    fn gen_total_outputs_const(&self) -> ItemConst {
        let outputs = &(self.outputs + self.interfaces.len());
        parse_quote!(const TOTAL_OUTPUTS: usize = #outputs;)
    }

    fn gen_buffer_len_const(&self) -> ItemConst {
        let b = &(self.mtu.bytes() as usize);
        parse_quote!(const BUF_LEN: usize = #b ;)
    }
}
