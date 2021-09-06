use proc_macro::TokenStream;
use proc_macro2::Span;
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
    let input_ident = &input.sig.ident;
    let inner_ident = syn::Ident::new(
        &format!("__do_{}", input_ident.to_string()),
        Span::call_site(),
    );

    if input.sig.asyncness.is_none() {
        return quote_spanned! {
            input_ident.span() => compile_error!("device handlers must be async");
        }
        .into();
    }
    if !input.attrs.is_empty() {
        return quote_spanned! {
            input_ident.span() => compile_error!("device handlers should not have other attributes");
        }.into();
    }

    let args_span = input.sig.inputs.span();
    if input.sig.inputs.len() != 1 {
        return quote_spanned! {
            args_span => compile_error!("device handlers take a single Arg struct");
        }
        .into();
    }
    let input_arg = match &input.sig.inputs[0] {
        syn::FnArg::Receiver(_) => {
            return quote_spanned! {
                args_span => compile_error!("device_handler directly takes a string as path");
            }
            .into()
        }
        syn::FnArg::Typed(ty) => ty,
    };
    let input_arg_ty = &input_arg.ty;
    let input_ret_ty = input.sig.output;
    let input_block = &input.block;

    let outer_fn = quote! {
        pub async fn #input_ident(body: Bytes) -> Result<Bytes, Error> {
            let args: #input_arg_ty = bincode::deserialize(body.as_ref()).map_err(|e| {
                actix_web::error::ErrorBadRequest(format!("Invalid argument: {}", e))
            })?;

            async fn #inner_ident(#input_arg) #input_ret_ty {
                #input_block
            }

            #inner_ident(args)
                .await
                .map(|r| Bytes::from(bincode::serialize(&r).unwrap()))
        }
    };

    let submit = quote! {
        inventory::submit!(DeviceHandler {
            path: #path,
            handler: |x| Box::pin(#input_ident(x))
        });
    };

    let mut out = outer_fn.into_token_stream();
    out.extend(submit);
    out.into()
}
