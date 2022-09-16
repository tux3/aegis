use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, LitStr, NestedMeta};

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
        &format!("__{input_fn_ident}_http_handler"),
        Span::call_site(),
    );
    let handler_fn_ident =
        syn::Ident::new(&format!("__{input_fn_ident}_handler"), Span::call_site());

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
    if input.sig.inputs.len() != 3 {
        return quote_spanned! {
            args_span => compile_error!("device handlers take two argument: A data handle, a device id, and a deserializable Arg struct");
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
    let dev_id_arg = &input.sig.inputs[1];
    let input_arg = match &input.sig.inputs[2] {
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
        pub async fn #http_fn_ident(axum::extract::State(db): axum::extract::State<sqlx::Pool<sqlx::Postgres>>,
                                    req: axum::http::Request<axum::body::Body>) -> Result<Bytes, crate::error::Error> {
            let dev_id = *req.extensions()
                             .get::<DeviceId>()
                             .expect("Missing device id in device request handler");

            inventory::submit!(handler_inventory::DeviceHandler {
                path: #path,
                http_handler: |db, req| Box::pin(#http_fn_ident(db, req)),
                handler: |db, id, arg| Box::pin(#handler_fn_ident(db, id, arg)),
            });

            pub async fn #handler_fn_ident(db: sqlx::PgPool, dev_id: DeviceId, body: Bytes) -> Result<Bytes, crate::error::Error> {
                let args: #input_arg_ty = bincode::deserialize(body.as_ref()).map_err(|e| {
                    crate::error::Error::Response(axum::http::StatusCode::BAD_REQUEST, format!("Invalid argument: {}", e))
                })?;
                let mut conn = db.acquire().await.map_err(|e| {
                    crate::error::Error::Response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB connection failed: {}", e))
                })?;

                async fn #input_fn_ident(#db_arg, #dev_id_arg, #input_arg) #input_ret_ty {
                    #input_block
                }

                #input_fn_ident(&mut *conn, dev_id, args)
                    .await
                    .map(|r| Bytes::from(bincode::serialize(&r).unwrap()))
                    .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
                        Some(db_err) => crate::error::Error::Response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, db_err.to_string()),
                        None => crate::error::Error::Response(axum::http::StatusCode::BAD_REQUEST, e.to_string()),
                    })
            }

            let body = hyper::body::to_bytes(req.into_body()).await.map_err(|e| {
                crate::error::Error::Response(axum::http::StatusCode::BAD_REQUEST, format!("Failed to read body: {}", e))
            })?;
            #handler_fn_ident(db, dev_id, body).await
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
    let input_ret_ty = input.sig.output;
    let input_block = &input.block;
    let input_fn_ident = &input.sig.ident;
    let http_fn_ident = syn::Ident::new(
        &format!("__{}_http_handler", &input_fn_ident),
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

    let args = &input.sig.inputs;
    let body_buf = quote!(
        use hyper::body::Buf;
        let body_buf = hyper::body::aggregate(req.into_body()).await.map_err(|e| {
            crate::error::Error::Response(axum::http::StatusCode::BAD_REQUEST, format!("Failed to read body: {}", e))
        })?;
    );
    let args_call = if args.len() == 1 {
        quote!(
            #body_buf
            let args: () = bincode::deserialize_from(body_buf.reader()).map_err(|_| {
                crate::error::Error::Response(axum::http::StatusCode::BAD_REQUEST, format!("Failed to deserialize payload"))
            })?;
            #input_fn_ident(&mut *conn)
        )
    } else if args.len() == 2 {
        let input_arg = match &args[1] {
            syn::FnArg::Receiver(_) => {
                return quote_spanned! {
                    args.span() => compile_error!("admin_handlers do not take a receiver");
                }
                .into()
            }
            syn::FnArg::Typed(ty) => ty,
        };
        let input_arg_ty = &input_arg.ty;
        quote!(
            #body_buf;
            let args: #input_arg_ty = bincode::deserialize_from(body_buf.reader()).map_err(|e| {
                crate::error::Error::Response(axum::http::StatusCode::BAD_REQUEST, format!("Invalid argument: {}", e))
            })?;
            #input_fn_ident(&mut *conn, args)
        )
    } else {
        return quote_spanned! {
            args.span() => compile_error!("admin_handlers take two argument: A db handle, and a deserializable Arg struct");
        }
            .into();
    };

    let outer_fn = quote! {
        pub async fn #http_fn_ident(axum::extract::State(db): axum::extract::State<sqlx::Pool<sqlx::Postgres>>,
                                    req: axum::http::Request<axum::body::Body>) -> Result<Bytes, crate::error::Error> {
            inventory::submit!(handler_inventory::AdminHandler {
                path: #path,
                http_handler: |db, req| Box::pin(#http_fn_ident(db, req)),
            });

            async fn #input_fn_ident(#args) #input_ret_ty {
                #input_block
            }

            let mut conn = db.acquire().await.map_err(|e| {
                crate::error::Error::Response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB connection failed: {}", e))
            })?;

            #args_call
                .await
                .map(|r| Bytes::from(bincode::serialize(&r).unwrap()))
                .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
                    Some(db_err) => crate::error::Error::Response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, db_err.to_string()),
                    None => crate::error::Error::Response(axum::http::StatusCode::BAD_REQUEST, e.to_string()),
                })
        }
    };
    outer_fn.into_token_stream().into()
}
