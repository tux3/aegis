use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, LitStr, NestedMeta, Type};

fn check_attr_args(args: &[NestedMeta]) -> Result<&LitStr, TokenStream> {
    if args.len() != 1 {
        return Err(quote! {
            compile_error!("device_handler takes a single path argument");
        }
        .into());
    }
    let path = match &args[0] {
        NestedMeta::Lit(Lit::Str(str)) => str,
        _ => {
            let span = args[0].span();
            return Err(quote_spanned! {
                span => compile_error!("device_handler directly takes a string as path");
            }
            .into());
        }
    };
    Ok(path)
}

#[proc_macro_attribute]
pub fn device_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as AttributeArgs);
    let path = match check_attr_args(&args) {
        Ok(path) => path,
        Err(e) => return e,
    };

    let input = parse_macro_input!(input as ItemFn);
    let input_fn_ident = &input.sig.ident;
    let http_fn_ident = syn::Ident::new(
        &format!("__{}_http_handler", input_fn_ident.to_string()),
        Span::call_site(),
    );
    let handler_fn_ident = syn::Ident::new(
        &format!("__{}_handler", input_fn_ident.to_string()),
        Span::call_site(),
    );

    if input.sig.asyncness.is_none() {
        return quote_spanned! {
            input_fn_ident.span() => compile_error!("device handlers must be async");
        }
        .into();
    }
    if !input.attrs.is_empty() {
        return quote_spanned! {
            input_fn_ident.span() => compile_error!("device handlers should not have other attributes");
        }.into();
    }

    let args_span = input.sig.inputs.span();
    if input.sig.inputs.len() != 2 {
        return quote_spanned! {
            args_span => compile_error!("device handlers take two argument: A data handle, and a deserializable Arg struct");
        }
            .into();
    }
    let db_arg = match &input.sig.inputs[0] {
        syn::FnArg::Receiver(_) => {
            return quote_spanned! {
                args_span => compile_error!("device_handlers do not take a receiver");
            }
            .into()
        }
        syn::FnArg::Typed(ty) => ty,
    };
    let db_arg_name = &db_arg.pat;
    let db_arg_ty = &*db_arg.ty;
    match db_arg_ty {
        Type::Path(_ty) => {},
        _ => return quote_spanned! {
                args_span => compile_error!("device_handler first argument must be an actix data handle");
            }
            .into(),
    };

    let input_arg = match &input.sig.inputs[1] {
        syn::FnArg::Receiver(_) => {
            return quote_spanned! {
                args_span => compile_error!("device_handlers do not take a receiver");
            }
            .into()
        }
        syn::FnArg::Typed(ty) => ty,
    };
    let input_arg_ty = &input_arg.ty;
    let input_ret_ty = input.sig.output;
    let input_block = &input.block;

    let outer_fn = quote! {
        pub async fn #http_fn_ident(req: actix_web::HttpRequest, body: Bytes) -> Result<Bytes, actix_web::Error> {
            inventory::submit!(handler_inventory::DeviceHandler {
                path: #path,
                http_handler: |req, arg| Box::pin(#http_fn_ident(req, arg)),
                handler: |db, arg| Box::pin(#handler_fn_ident(db, arg)),
            });

            let db = req
                .app_data::<#db_arg_ty>()
                .cloned()
                .expect("Missing db app data in device request handler");

            pub async fn #handler_fn_ident(#db_arg, body: Bytes) -> Result<Bytes, actix_web::Error> {
                let args: #input_arg_ty = bincode::deserialize(body.as_ref()).map_err(|e| {
                    actix_web::error::ErrorBadRequest(format!("Invalid argument: {}", e))
                })?;

                async fn #input_fn_ident(#db_arg, #input_arg) #input_ret_ty {
                    #input_block
                }

                #input_fn_ident(#db_arg_name, args)
                    .await
                    .map(|r| Bytes::from(bincode::serialize(&r).unwrap()))
                    .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
                        Some(db_err) => actix_web::error::ErrorInternalServerError(format!("{}", db_err)),
                        None => actix_web::error::ErrorBadRequest(e),
                    })
            }
            #handler_fn_ident(db, body).await
        }
    };
    outer_fn.into_token_stream().into()
}

#[proc_macro_attribute]
pub fn admin_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as AttributeArgs);
    let path = match check_attr_args(&args) {
        Ok(path) => path,
        Err(e) => return e,
    };

    let input = parse_macro_input!(input as ItemFn);
    let input_fn_ident = &input.sig.ident;
    let http_fn_ident = syn::Ident::new(
        &format!("__{}_http_handler", input_fn_ident.to_string()),
        Span::call_site(),
    );

    if input.sig.asyncness.is_none() {
        return quote_spanned! {
            input_fn_ident.span() => compile_error!("admin_handlers must be async");
        }
        .into();
    }
    if !input.attrs.is_empty() {
        return quote_spanned! {
            input_fn_ident.span() => compile_error!("admin_handlers should not have other attributes");
        }.into();
    }

    let args_span = input.sig.inputs.span();
    if input.sig.inputs.len() != 2 {
        return quote_spanned! {
            args_span => compile_error!("admin_handlers take two argument: A db handle, and a deserializable Arg struct");
        }
            .into();
    }
    let db_arg = match &input.sig.inputs[0] {
        syn::FnArg::Receiver(_) => {
            return quote_spanned! {
                args_span => compile_error!("admin_handlers do not take a receiver");
            }
            .into()
        }
        syn::FnArg::Typed(ty) => ty,
    };

    let input_arg = match &input.sig.inputs[1] {
        syn::FnArg::Receiver(_) => {
            return quote_spanned! {
                args_span => compile_error!("admin_handlers do not take a receiver");
            }
            .into()
        }
        syn::FnArg::Typed(ty) => ty,
    };
    let input_arg_ty = &input_arg.ty;
    let input_ret_ty = input.sig.output;
    let input_block = &input.block;

    let outer_fn = quote! {
        pub async fn #http_fn_ident(req: actix_web::HttpRequest, body: Bytes) -> Result<Bytes, actix_web::Error> {
            inventory::submit!(handler_inventory::AdminHandler {
                path: #path,
                http_handler: |req, arg| Box::pin(#http_fn_ident(req, arg)),
            });

            let db = req
                .app_data::<sqlx::PgPool>()
                .cloned()
                .expect("Missing db app data in admin request handler");
            let mut conn = db.acquire().await.map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("DB connection failed: {}", e))
            })?;
            let args: #input_arg_ty = bincode::deserialize(body.as_ref()).map_err(|e| {
                actix_web::error::ErrorBadRequest(format!("Invalid argument: {}", e))
            })?;

            async fn #input_fn_ident(#db_arg, #input_arg) #input_ret_ty {
                #input_block
            }

            #input_fn_ident(&mut *conn, args)
                .await
                .map(|r| Bytes::from(bincode::serialize(&r).unwrap()))
                .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
                    Some(db_err) => actix_web::error::ErrorInternalServerError(format!("{}", db_err)),
                    None => actix_web::error::ErrorBadRequest(e),
                })
        }
    };
    outer_fn.into_token_stream().into()
}
