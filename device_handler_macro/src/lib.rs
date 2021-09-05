use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, NestedMeta};

#[proc_macro_attribute]
pub fn device_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as AttributeArgs);
    if args.len() != 1 {
        return quote! {
            compile_error!("device_handler takes a single path argument");
        }
        .into();
    }
    let path = match &args[0] {
        NestedMeta::Lit(Lit::Str(str)) => str,
        _ => {
            let span = args[0].span();
            return quote_spanned! {
                span => compile_error!("device_handler directly takes a string as path");
            }
            .into();
        }
    };

    let input = parse_macro_input!(input as ItemFn);
    let handler_ident = &input.sig.ident;
    let submit = quote! {
        inventory::submit!(DeviceHandler {
            path: #path,
            handler: |x| Box::pin(#handler_ident(x))
        });
    };

    let mut out = input.into_token_stream();
    out.extend(submit);
    out.into()
}
