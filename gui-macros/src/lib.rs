mod dropdown_options;
mod wrap_property;

use proc_macro::TokenStream;

#[proc_macro_derive(DropdownOptions)]
pub fn derive_enum_drowndown_options(item: TokenStream) -> TokenStream {
    dropdown_options::derive_enum_dropdown_options_impl(item)
}

#[proc_macro]
pub fn wrap_property(item: TokenStream) -> TokenStream {
    wrap_property::wrap_property_impl(item)
}
