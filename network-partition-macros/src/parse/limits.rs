use darling::FromAttributes;
use syn::Item;

use crate::{
    attrs::{no_struct_body, remove_attr, MayFromAttributes},
    types::WrappedByteSize,
};

use crate::attrs::contains_attr;

#[derive(Debug, FromAttributes)]
#[darling(attributes(limits))]
pub struct Limits {
    #[darling(default)]
    pub inputs: usize,
    pub mtu: WrappedByteSize,
    #[darling(default)]
    pub outputs: usize,
}

impl MayFromAttributes for Limits {
    fn may_from_attributes(
        _ident: syn::Ident,
        attrs: &mut Vec<syn::Attribute>,
    ) -> Option<darling::Result<Self>> {
        if !contains_attr(attrs, "limits") {
            return None;
        }
        let l = Some(Self::from_attributes(attrs.as_slice()));
        Some(remove_attr(attrs, "limits"))?.ok();
        l
    }
}

impl Limits {
    pub(crate) fn from_content(content: &mut Vec<Item>) -> darling::Result<Self> {
        let mut limits: Option<Self> = None;
        let mut acc = darling::Error::accumulator();
        let c: Vec<Item> = content
            .iter_mut()
            .flat_map(|item| {
                if let Item::Struct(item) = item {
                    let res = Self::may_from_attributes(item.ident.clone(), &mut item.attrs);
                    match res {
                        Some(Ok(res)) => {
                            if let Err(e) = no_struct_body(item) {
                                acc.push(e);
                            } else if limits.is_some() {
                                acc.push(syn::Error::new_spanned(item, "Duplicate").into())
                            } else {
                                limits.replace(res);
                            }
                            None
                        }
                        Some(Err(e)) => {
                            acc.push(e);
                            None
                        }
                        None => Some(Item::Struct(item.clone())),
                    }
                } else {
                    Some(item.clone())
                }
            })
            .collect();
        content.clear();
        content.clone_from(&c);
        acc.finish()?;
        limits.ok_or_else(|| syn::Error::new_spanned(content.last(), "Missing `limits`").into())
    }
}
