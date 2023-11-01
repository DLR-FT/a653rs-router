use syn::{spanned::Spanned, ItemMod, Path};
use wrapped_types::WrappedByteSize;

use super::{interface::Interface, limits::Limits};

#[derive(Debug, Clone)]
pub struct Router {
    pub inputs: usize,
    pub interfaces: Vec<Interface>,
    pub mtu: WrappedByteSize,
    pub name: proc_macro2::Ident,
    pub outputs: usize,
    pub scheduler: Path,
}

impl Router {
    pub fn parse(scheduler: &Path, module: &mut ItemMod) -> syn::Result<Router> {
        let root = module.span();
        let (_, content) = module
            .content
            .as_mut()
            .ok_or_else(|| syn::Error::new(root, "Missing content"))?;

        let limits = Limits::from_content(content)?;

        let interfaces = Interface::from_content(content, limits.mtu.bytes() as usize)?;

        Ok(Router {
            name: module.ident.clone(),
            scheduler: scheduler.clone(),
            interfaces,
            inputs: limits.inputs,
            outputs: limits.outputs,
            mtu: limits.mtu,
        })
    }
}
