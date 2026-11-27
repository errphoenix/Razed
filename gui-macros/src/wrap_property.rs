use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse_quote;

use crate::wrap_property::ident_case_transform::PascalCaseExt;

pub(crate) fn wrap_property_impl(item: proc_macro::TokenStream) -> TokenStream {
    let mut input = syn::parse_macro_input!(item as syn::Block);
    let mut out = quote! {};
    for stmt in &mut input.stmts {
        match stmt {
            syn::Stmt::Item(itm) => match itm {
                syn::Item::Const(item_const) => {
                    handle_notcopy_attrib_if_defined(&mut item_const.attrs);
                    let map_fn = match extract_map_attrib(&mut item_const.attrs) {
                        Err(err) => return err,
                        Ok(map_fn) => map_fn,
                    };

                    let ty = &item_const.ty;
                    let init_fn = if let Some(map_fn_v) = map_fn {
                        quote! {
                            pub fn new(value: impl Into<#ty>) -> Self {
                                Self(gui::Property::with_value_mapped(value, #map_fn_v))
                            }
                        }
                    } else {
                        quote! {
                            pub const fn new(value: #ty) -> Self {
                                Self(gui::Property::new_const(value))
                            }
                        }
                    };

                    let attribs = &item_const.attrs;
                    let expr = &item_const.expr;
                    let ident_upper = &item_const.ident;
                    let ident_lower = lowercase_ident(&ident_upper);
                    let ident_pascal = pascalcase_ident(&ident_lower);

                    let ident_dotted = separators_ident(&ident_lower.to_string(), "-");
                    let prop_id_ident =
                        syn::Ident::new(&format!("PROP_ID_{ident_upper}"), Span::call_site());
                    let to_hash_id = format!("#macro-prop/{ident_dotted}/value");

                    out.extend(quote! {
                        pub const #prop_id_ident: janus::StringHash = janus::hash_string(&#to_hash_id);

                        #(#attribs)*
                        #[derive(Clone, Debug)]
                        pub struct #ident_pascal(gui::Property<#ty, { #prop_id_ident.as_u64() }>);
                        impl std::default::Default for #ident_pascal {
                            fn default() -> Self {
                                Self::new(#expr)
                            }
                        }
                        impl #ident_pascal {
                            #init_fn

                            pub const fn name() -> janus::StringHash {
                                #prop_id_ident
                            }

                            pub fn original_name() -> &'static str {
                                &#to_hash_id
                            }

                            pub const fn get(&self) -> &#ty {
                                self.0.get()
                            }

                            pub const fn get_mut(&mut self) -> &mut #ty {
                                self.0.get_mut()
                            }

                            pub const fn inner(&self) -> &gui::Property<#ty, { #prop_id_ident.as_u64() }> { &self.0 }
                            pub const fn inner_mut(&mut self) -> &mut gui::Property<#ty, { #prop_id_ident.as_u64() }> { &mut self.0 }
                        }
                        impl std::ops::Deref for #ident_pascal {
                            type Target = gui::Property<#ty, { #prop_id_ident.as_u64() }>;

                            fn deref(&self) -> &Self::Target {
                                self.inner()
                            }
                        }
                        impl std::ops::DerefMut for #ident_pascal {
                            fn deref_mut(&mut self) -> &mut Self::Target {
                                self.inner_mut()
                            }
                        }
                    });
                }
                _ => {
                    return syn::Error::new_spanned(
                        &input,
                        "wrap_property macro only supports const declarations",
                    )
                    .to_compile_error()
                    .into();
                }
            },
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "wrap_property block must contain statements",
                )
                .to_compile_error()
                .into();
            }
        }
    }
    out.into()
}

fn extract_map_range(expr: syn::ExprRange) -> syn::Expr {
    let fallback_min = Box::<syn::Expr>::new(parse_quote!(0.0));
    let fallback_max = Box::<syn::Expr>::new(parse_quote!(1.0));

    let min = expr.start.unwrap_or(fallback_min);
    let max = expr.end.unwrap_or(fallback_max);

    parse_quote!( |v| *v = #min + (#max - #min) * *v )
}

fn extract_map_attrib(attribs: &mut Vec<syn::Attribute>) -> Result<Option<syn::Expr>, TokenStream> {
    let mut result = Ok(None);
    attribs.retain(|attrib| {
        if attrib.path().is_ident("map") {
            match attrib.parse_args::<syn::Expr>() {
                Ok(expr) => {
                    if matches!(expr, syn::Expr::Closure(_)) {
                        result = Ok(Some(expr));
                    } else {
                        let parse_result = attrib.parse_nested_meta(|meta| {
                            if meta.path.is_ident("range") {
                                let value = meta.value()?;
                                let range: syn::ExprRange = value.parse()?;
                                result = Ok(Some(extract_map_range(range)));
                            }
                            Ok(())
                        });
                        if let Err(err) = parse_result {
                            result = Err(syn::Error::new_spanned(
                                attrib,
                                format!("invalid mapping descriptor: {err}"),
                            )
                            .to_compile_error()
                            .into())
                        }
                    }
                }
                Err(err) => {
                    result = Err(syn::Error::new_spanned(
                        attrib,
                        format!("invalid mapping descriptor: {err}"),
                    )
                    .to_compile_error()
                    .into())
                }
            }
            false
        } else {
            true
        }
    });
    result
}

fn handle_notcopy_attrib_if_defined(attribs: &mut Vec<syn::Attribute>) {
    let not_copy = attribs.iter().any(|attr| attr.path().is_ident("not_copy"));
    attribs.retain(|attr| !attr.path().is_ident("not_copy"));
    if !not_copy {
        attribs.push(syn::parse_quote!(#[derive(Copy)]));
    }
}

fn lowercase_ident(ident: &syn::Ident) -> syn::Ident {
    let string = ident.to_string();
    syn::Ident::new(&string.to_lowercase(), Span::call_site())
}
fn pascalcase_ident(ident: &syn::Ident) -> syn::Ident {
    let string = ident.to_string();
    syn::Ident::new(&string.to_pascal_case(), Span::call_site())
}
fn separators_ident(ident: &str, new_sep: &str) -> String {
    ident.to_string().replace("_", new_sep)
}

/// This section is from the `casey` crate's `traits` module.
mod ident_case_transform {
    const SEPARATORS: &str = "-_";

    pub trait PascalCaseExt {
        fn to_pascal_case(&self) -> String;
    }
    impl PascalCaseExt for String {
        fn to_pascal_case(&self) -> String {
            let mut s = String::new();
            let mut capitalise_next = true; // always on first char

            let mut char_stream = self.chars().peekable();
            while let Some(current_char) = char_stream.next() {
                let next_char = char_stream.peek();
                if next_char.is_none() {
                    // `current_char` is last in the stream
                    s.push(current_char.to_lowercase().next().unwrap());
                    break;
                }

                if SEPARATORS.contains(current_char) {
                    capitalise_next = true;
                    continue;
                }
                if current_char.is_numeric() {
                    capitalise_next = true;
                    s.push(current_char);
                    continue;
                }

                if capitalise_next {
                    s.push(current_char.to_uppercase().next().unwrap());
                    capitalise_next = false;
                    continue;
                }

                // lowercase this char if followed by another uppercase or punctuation
                // e.g. AA => aA, A- => a-
                // has the affect of transforming: 'ABCDe' into 'AbcDe'
                if current_char.is_ascii_uppercase()
                    && next_char.is_some_and(|x| {
                        x.is_numeric() || x.is_uppercase() || x.is_ascii_punctuation()
                    })
                {
                    s.push(current_char.to_lowercase().next().unwrap());
                } else {
                    s.push(current_char);
                }
            }
            s
        }
    }
}
