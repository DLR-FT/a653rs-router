pub mod interface;
pub mod router;

use proc_macro2::TokenStream;
use syn::ItemMod;

pub trait GenMod {
    fn gen_mod(&self) -> syn::Result<ItemMod>;
}

pub trait GenerateStream {
    fn gen_stream(&self, content: &mut ItemMod) -> syn::Result<TokenStream>;
}
