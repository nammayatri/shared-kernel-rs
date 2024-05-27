/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Expr, ItemEnum, ItemFn, ItemStruct, Lit};

#[proc_macro_attribute]
pub fn measure_duration(_: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let function_body = &input_fn.block;
    let fn_name = &input_fn.sig.ident;
    let args = &input_fn.sig.inputs;
    let return_type = &input_fn.sig.output;
    let generics = &input_fn.sig.generics;
    let where_clause = &input_fn.sig.generics.where_clause;
    let is_async = input_fn.sig.asyncness.is_some();

    let expanded = if is_async {
        quote! {
            pub async fn #fn_name #generics(#args) #return_type #where_clause {
                let start_time = std::time::Instant::now();
                let result = #function_body;
                measure_latency_duration!(stringify!(#fn_name), start_time);
                let elapsed_time = start_time.elapsed();
                let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
                debug!("Function: {} | Duration (ms): {}", stringify!(#fn_name), elapsed_ms);
                result
            }
        }
    } else {
        quote! {
            pub fn #fn_name #generics(#args) #return_type #where_clause {
                let start_time = std::time::Instant::now();
                let result = #function_body;
                measure_latency_duration!(stringify!(#fn_name), start_time);
                let elapsed_time = start_time.elapsed();
                let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
                debug!("Function: {} | Duration (ms): {}", stringify!(#fn_name), elapsed_ms);
                result
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn measure_duration_block(input: TokenStream) -> TokenStream {
    let cloned_input = input.clone();
    let block = parse_macro_input!(input as syn::Block);
    let input = match parse_macro_input!(cloned_input as Expr) {
        Expr::Lit(expr_lit) => match expr_lit.lit {
            Lit::Str(lit_str) => Some(lit_str.value()),
            _ => None,
        },
        _ => None,
    };

    let expanded = if let Some(name) = input {
        quote! {
            {
                let start_time = std::time::Instant::now();
                let result = { #block };
                measure_latency_duration!(stringify!(#name), start_time);
                let elapsed_time = start_time.elapsed();
                let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
                debug!("Function: {} | Duration (ms): {}", stringify!(#name), elapsed_ms);
                result
            }
        }
    } else {
        quote! {
            {
                let start_time = std::time::Instant::now();
                let result = { #block };
                measure_latency_duration!(stringify!("unknown_block_name"), start_time);
                let elapsed_time = start_time.elapsed();
                let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
                debug!("Function: {} | Duration (ms): {}", stringify!("unknown_block_name"), elapsed_ms);
                result
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn generate_flamegraph(_: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let function_body = &input_fn.block;
    let fn_name = &input_fn.sig.ident;
    let args = &input_fn.sig.inputs;
    let return_type = &input_fn.sig.output;
    let visibility = &input_fn.vis;
    let asyncness = input_fn.sig.asyncness;

    let function_start = match (asyncness, visibility) {
        (Some(_), _) => quote! { pub async fn },
        (None, syn::Visibility::Public(_)) => quote! { pub fn },
        (None, _) => quote! { fn },
    };

    let expanded = quote! {
        #function_start #fn_name(#args) #return_type {
            let guard = pprof::ProfilerGuard::new(1000).unwrap();
            let result = #function_body;
            if let Ok(report) = guard.report().build() {
                std::fs::create_dir_all("./profiling").unwrap();
                let flamegraph_file = std::fs::File::create(format!("./profiling/{}-flamegraph.svg", stringify!(#fn_name))).unwrap();
                let mut prof_file = std::fs::File::create(format!("./profiling/{}-profiling.prof", stringify!(#fn_name))).unwrap();
                let _ = report
                    .flamegraph(flamegraph_file)
                    .map_err(|err| err.to_string());
                std::io::Write::write_all(&mut prof_file, format!("{:?}", report).as_bytes()).unwrap();
            };
            result
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn add_error(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemEnum);
    let enum_name = &input.ident;

    let variants = input.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_screaming_snake_case = convert_to_snake_case(variant_name.to_string());
        quote! {
            #[error(#variant_screaming_snake_case)]
            #variant,
        }
    });

    let expanded = quote! {
        #[derive(Debug, thiserror::Error)]
        pub enum #enum_name {
            #(#variants)*
        }
    };

    TokenStream::from(expanded)
}

fn convert_to_snake_case(input: String) -> String {
    let mut result = String::new();
    let mut last_char_was_upper = false;

    for c in input.chars() {
        if c.is_uppercase() {
            if !last_char_was_upper && !result.is_empty() {
                result.push('_');
            }
            result.push(c.to_ascii_uppercase());
            last_char_was_upper = true;
        } else {
            result.push(c.to_ascii_uppercase());
            last_char_was_upper = false;
        }
    }

    result
}

#[proc_macro_attribute]
pub fn impl_getter(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    let struct_name = &input.ident;
    let inner_type = match &input.fields {
        syn::Fields::Named(fields) => {
            if let Some(field) = fields.named.first() {
                &field.ty
            } else {
                return syn::Error::new_spanned(&input, "Struct must have at least one field")
                    .to_compile_error()
                    .into();
            }
        }
        syn::Fields::Unnamed(fields) => {
            if let Some(field) = fields.unnamed.first() {
                &field.ty
            } else {
                return syn::Error::new_spanned(&input, "Struct must have at least one field")
                    .to_compile_error()
                    .into();
            }
        }
        syn::Fields::Unit => {
            return syn::Error::new_spanned(&input, "Struct must have at least one field")
                .to_compile_error()
                .into();
        }
    };

    let gen = quote! {
        pub struct #struct_name(pub #inner_type);

        impl #struct_name {
            pub fn inner(&self) -> #inner_type {
                self.0.to_owned()
            }
        }
    };

    gen.into()
}
