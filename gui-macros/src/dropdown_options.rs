use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

pub(crate) fn derive_enum_dropdown_options_impl(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    let enum_ident = &input.ident;

    match &input.data {
        syn::Data::Enum(syn::DataEnum { variants, .. }) => {
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
                        let discriminant = match expr_extract_integer(&enum_discr.1) {
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

fn expr_extract_integer(expr: &syn::Expr) -> Result<usize, TokenStream> {
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
