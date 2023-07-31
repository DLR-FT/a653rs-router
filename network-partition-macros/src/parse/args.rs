use syn::{parse::Parse, token::Bracket, Expr, ExprArray, Token, TypePath};

pub struct RunArgs {
    pub router: TypePath,
    pub time_source: Expr,
    pub router_config: Expr,
    pub inputs: Option<ExprArray>,
    pub outputs: Option<ExprArray>,
}

impl Parse for RunArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let router = input.parse()?;
        input.parse::<Token![,]>()?;
        let time_source = input.parse()?;
        input.parse::<Token![,]>()?;
        let router_config = input.parse()?;
        let inputs: Option<_> = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.peek(Bracket) {
                let o = input.parse()?;
                Some(o)
            } else {
                None
            }
        } else {
            None
        };
        let outputs: Option<_> = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.peek(Bracket) {
                let o = input.parse()?;
                Some(o)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            router,
            time_source,
            router_config,
            inputs,
            outputs,
        })
    }
}
