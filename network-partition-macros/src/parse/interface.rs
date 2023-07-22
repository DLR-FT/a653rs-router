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
    fn check(self, max_mtu: usize) -> darling::Result<Self> {
        let i_mtu = self.mtu.bytes() as usize;
        if i_mtu > max_mtu {
            Err(darling::Error::custom(format!(
                "Interface MTU = {i_mtu:?} is larger than router maximum MTU = {max_mtu}"
            )))
        } else {
            Ok(self)
        }
    }

    fn from_item(item: &mut Item, max_mtu: usize) -> darling::Result<ParseResult> {
        if let Item::Struct(item) = item {
            let parsed = Self::may_from_attributes(item.ident.clone(), &mut item.attrs);
            match parsed {
                Some(Ok(res)) => {
                    no_struct_body(item)?;
                    match res.check(max_mtu) {
                        Ok(i) => Ok(ParseResult::Interface(i)),
                        Err(e) => Err(e.with_span(item)),
                    }
                }
                Some(Err(e)) => Err(e),
                None => Ok(ParseResult::Unchanged(Item::Struct(item.clone()))),
            }
        } else {
            Ok(ParseResult::Unchanged(item.clone()))
        }
    }

    pub fn from_content(items: &mut Vec<Item>, max_mtu: usize) -> darling::Result<Vec<Self>> {
        let mut interfaces: Vec<Self> = vec![];
        let mut acc = darling::Error::accumulator();
        let mut unchanged: Vec<Item> = vec![];
        for item in items.iter_mut() {
            match Self::from_item(item, max_mtu) {
                Ok(ParseResult::Unchanged(item)) => {
                    unchanged.push(item);
                }
                Ok(ParseResult::Interface(intf)) => {
                    interfaces.push(intf);
                }
                Err(e) => {
                    acc.push(e);
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
