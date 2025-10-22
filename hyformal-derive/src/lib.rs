use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input};

struct SymbolList {
    pub symbols: Vec<syn::Ident>,
}

impl Parse for SymbolList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let punctuated =
            syn::punctuated::Punctuated::<syn::Ident, syn::token::Comma>::parse_terminated(input)?;

        Ok(SymbolList {
            symbols: punctuated.into_iter().collect(),
        })
    }
}

#[proc_macro]
pub fn internal_symbols(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SymbolList);

    // Generate a new line for each symbol with the following content
    // let $symbol = hyformal::prelude::InlineVariable::new(hyformal::prelude::Variable::Internal(generated_index))
    let mut generated_lines = Vec::new();
    for (generated_index, symbol) in input.symbols.iter().enumerate() {
        let generated_index = generated_index as u32;
        let line = quote! {
            let #symbol = hyformal::prelude::InlineVariable::new(hyformal::prelude::Variable::Internal(#generated_index));
        };
        generated_lines.push(line);
    }

    let expanded = quote! {
        #(#generated_lines)*
    };
    TokenStream::from(expanded)
}
