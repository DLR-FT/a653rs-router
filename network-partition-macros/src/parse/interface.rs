use darling::{FromAttributes, FromMeta};
use syn::{Attribute, Expr, Ident, Item};

use crate::{
    attrs::{contains_attr, no_struct_body, remove_attr, MayFromAttributes},
    types::WrappedByteSize,
};

#[derive(Debug, Clone)]
pub struct WrappedPath {
    pub path: syn::Path,
}

impl WrappedPath {
    fn from_path(path: &syn::Path) -> Result<WrappedPath, darling::Error> {
        Ok(Self { path: path.clone() })
    }
}

impl FromMeta for WrappedPath {
    fn from_expr(expr: &syn::Expr) -> darling::Result<Self> {
        match *expr {
            Expr::Lit(ref lit) => Self::from_value(&lit.lit),
            Expr::Group(ref group) => Self::from_expr(&group.expr),
            Expr::Path(ref path) => Self::from_path(&path.path),
            _ => Err(darling::Error::unexpected_expr_type(expr)),
        }
        .map_err(|e| e.with_span(expr))
    }
}

#[derive(Debug, FromAttributes, Clone)]
#[darling(attributes(interface))]
pub struct Interface {
    #[darling(default = "String::default")]
    pub name: String,
    pub interface_type: WrappedPath,
    pub rate: WrappedByteSize,
    pub mtu: WrappedByteSize,
    pub source: String,
    pub destination: String,
}

enum ParseResult {
    Interface(Interface),
    Unchanged(Item),
}

impl Interface {
    pub fn from_content(items: &mut Vec<Item>) -> darling::Result<Vec<Self>> {
        let mut interfaces: Vec<Self> = vec![];
        let mut acc = darling::Error::accumulator();
        let c: Vec<Item> = items
            .iter_mut()
            .flat_map(|item| {
                if let Item::Struct(item) = item {
                    let res = Self::may_from_attributes(item.ident.clone(), &mut item.attrs);
                    match res {
                        Some(Ok(res)) => {
                            if let Err(e) = no_struct_body(item) {
                                acc.push(e);
                            } else {
                                interfaces.push(res);
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
            }
        }
        items.clear();
        items.clone_from(&unchanged);
        acc.finish()?;
        Ok(interfaces)
    }
}

impl MayFromAttributes for Interface {
    fn may_from_attributes(
        ident: Ident,
        attrs: &mut Vec<Attribute>,
    ) -> Option<darling::Result<Self>> {
        if !contains_attr(attrs, "interface") {
            return None;
        }
        let i = Self::from_attributes(attrs.as_slice()).map(|mut i| {
            i.name = ident.to_string();
            i
        });
        Some(remove_attr(attrs, "interface"))?.ok();
        Some(i)
    }
}
