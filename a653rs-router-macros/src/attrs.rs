use syn::{spanned::Spanned, Attribute, Ident, ItemStruct};

pub fn no_struct_body(i: &ItemStruct) -> darling::Result<()> {
    if i.fields.is_empty() {
        Ok(())
    } else {
        Err(darling::Error::custom("Interface struct has a body").with_span(&i.span()))
    }
}

pub trait MayFromAttributes: Sized {
    fn may_from_attributes(
        ident: Ident,
        attrs: &mut Vec<Attribute>,
    ) -> Option<darling::Result<Self>>;
}

pub fn contains_attr(attrs: &[Attribute], attr: &str) -> bool {
    attrs
        .iter()
        .flat_map(|a| a.meta.path().get_ident().cloned())
        .any(|i| i.to_string().eq(attr))
}

pub fn remove_attr(attrs: &mut Vec<Attribute>, attr: &str) -> syn::Result<()> {
    let attr = syn::parse_str::<Ident>(attr)?;
    attrs.retain(|a| {
        a.path()
            .segments
            .first()
            .map_or_else(|| true, |p| !p.ident.eq(&attr))
    });
    Ok(())
}
