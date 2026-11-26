use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DataEnum, Ident, parse_macro_input};

use crate::ident_case_transform::PascalCaseExt;

fn extract_integer(expr: &syn::Expr) -> Result<usize, TokenStream> {
    match expr {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(discriminant_int) => discriminant_int
                .base10_parse()
                .map_err(|err| err.to_compile_error().into()),
            _ => {
                return Err(
                    syn::Error::new_spanned(expr, "enum discriminant must be an integer")
                        .to_compile_error()
                        .into(),
                );
            }
        },
        _ => {
            return Err(
                syn::Error::new_spanned(expr, "enum discriminant must be literal")
                    .to_compile_error()
                    .into(),
            );
        }
    }
}

#[proc_macro_derive(DropdownOptions)]
pub fn derive_enum_drowndown_options(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    let enum_ident = &input.ident;

    match &input.data {
        Data::Enum(DataEnum { variants, .. }) => {
            let variant_count = variants.len();
            let variant_names = variants.iter().map(|v| &v.ident).collect::<Vec<_>>();

            let mut default = None;
            let discriminants = {
                const MAX_VALUE: usize = 16;
                let mut avail = [true; MAX_VALUE];
                let mut discriminants = vec![0u32; variants.len()];
                let mut not_found = Vec::new();
                for (i, variant) in variants.iter().enumerate() {
                    if let Some(enum_discr) = &variant.discriminant {
                        let discriminant = match extract_integer(&enum_discr.1) {
                            Ok(discriminant) => discriminant,
                            Err(err) => return err,
                        };

                        if discriminant >= MAX_VALUE {
                            return syn::Error::new_spanned(
                                &input,
                                "discriminant must be within the interval [0; 15]",
                            )
                            .to_compile_error()
                            .into();
                        }

                        avail[discriminant] = false;
                        discriminants[i] = discriminant as u32;
                    } else {
                        not_found.push(i);
                    };
                }

                // no discriminants provided in enum for these variants,
                // just assign them to empty gaps in the discriminants list
                for i in not_found {
                    let mut discriminant = 0;
                    let mut j = 0;
                    while j < MAX_VALUE {
                        if avail[i] {
                            discriminant = j;
                            break;
                        }
                        j += 1;
                    }
                    avail[discriminant] = false;
                    discriminants[i] = discriminant as u32;
                }

                // all variants have a discriminant, now look for default
                'def_lookup: for (i, variant) in variants.iter().enumerate() {
                    for attr in &variant.attrs {
                        if attr.path().is_ident("default") {
                            default = Some(discriminants[i]);
                            break 'def_lookup;
                        }
                    }
                }

                discriminants
            };

            // if no default is given just use the first variant
            let default = default.unwrap_or(discriminants[0]);

            quote! {
                impl gui::dropdown::HasDropdownEntries for #enum_ident {
                    fn discriminant(&self) -> u32 {
                        *self as _
                    }

                    fn entries() -> [gui::DropdownEntry; gui::dropdown::MAX_ENTRIES] {
                        let base_entries = Self::dropdown_entries();
                        let mut entries = [gui::DropdownEntry::empty(); gui::dropdown::MAX_ENTRIES];
                        let mut i = 0;
                        while i < gui::dropdown::MAX_ENTRIES && i < #variant_count {
                            entries[i] = base_entries[i];
                            i += 1;
                        }
                        entries
                    }

                    fn default() -> u32 {
                        #default
                    }
                }
                impl From<gui::EnvValue> for #enum_ident {
                    fn from(value: gui::EnvValue) -> Self {
                        let fallback = <Self as gui::dropdown::HasDropdownEntries>::default();
                        let int = value.as_integer().unwrap_or(fallback as i32) as u32;
                        Self::try_from_discriminant(int).unwrap_or_default()
                    }
                }
                impl From<#enum_ident> for gui::EnvValue {
                    fn from(value: #enum_ident) -> Self {
                        value.discriminant().into()
                    }
                }
                impl #enum_ident {
                    pub const fn dropdown_entries() -> [gui::DropdownEntry; #variant_count] {
                        let mut entries = [gui::DropdownEntry::empty(); #variant_count];
                        let mut i = 0;
                        #(
                            entries[i] = gui::DropdownEntry::new(stringify!(#variant_names), #enum_ident::#variant_names as _);
                            i += 1;
                        )*
                        entries
                    }

                    pub const fn discriminant(&self) -> u32 {
                        *self as u32
                    }

                    pub const fn try_from_discriminant(discriminant: u32) -> Option<Self> {
                        match discriminant {
                            #(
                                #discriminants => Some(#enum_ident::#variant_names),
                            )*
                            _ => None
                        }
                    }

                    pub const fn from_discriminant(discriminant: u32) -> Self {
                        Self::try_from_discriminant(discriminant).unwrap()
                    }
                }
            }
            .into()
        }
        _ => syn::Error::new_spanned(&input, "DropdownOptions derive macro only supports enums")
            .to_compile_error()
            .into(),
    }
}

#[proc_macro]
pub fn wrap_property(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::Block);
    let mut out = quote! {};
    for stmt in &input.stmts {
        match stmt {
            syn::Stmt::Item(itm) => match itm {
                syn::Item::Const(item_const) => {
                    let attribs = &item_const.attrs;
                    let ty = &item_const.ty;
                    let expr = &item_const.expr;
                    let ident_upper = &item_const.ident;
                    let ident_lower = lowercase_ident(&ident_upper);
                    let ident_pascal = pascalcase_ident(&ident_lower);

                    let ident_dotted = separators_ident(&ident_lower.to_string(), "-");
                    let prop_id_ident =
                        Ident::new(&format!("PROP_ID_{ident_upper}"), Span::call_site());
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
                            pub const fn new(value: #ty) -> Self {
                                Self(gui::Property::new_const(value))
                            }

                            pub const fn name() -> janus::StringHash {
                                #prop_id_ident
                            }

                            /// Note that this allocates a new `String`.
                            pub fn original_name() -> String {
                                #to_hash_id.to_string()
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

fn lowercase_ident(ident: &Ident) -> Ident {
    let string = ident.to_string();
    Ident::new(&string.to_lowercase(), Span::call_site())
}
fn pascalcase_ident(ident: &Ident) -> Ident {
    let string = ident.to_string();
    Ident::new(&string.to_pascal_case(), Span::call_site())
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
