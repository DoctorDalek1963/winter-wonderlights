//! Handle deriving traits.

use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive the `BaseEffect` trait.
pub fn derive_base_effect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;
    let sealed_impl = create_sealed_impl(&struct_name);

    quote! {
        #sealed_impl

        impl crate::traits::BaseEffect for #struct_name {
            fn effect_name() -> &'static str {
                stringify!(#struct_name)
            }

            fn save_to_file(&self) {
                self.config.save_to_file(&<Self as crate::traits::Effect>::config_filename());
            }

            fn from_file() -> Self {
                Self {
                    config: <Self as crate::traits::Effect>::Config::from_file(
                        &<Self as crate::traits::Effect>::config_filename()
                    ),
                    ..<Self as ::std::default::Default>::default()
                }
            }
        }
    }
    .into()
}

/// Derive the `Sealed` trait.
pub fn derive_sealed(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    create_sealed_impl(&input.ident).into()
}

/// Create an implementation of `Sealed` for a type with the given name.
fn create_sealed_impl(name: &Ident2) -> TokenStream2 {
    quote! {
        impl crate::traits::private::Sealed for #name {}
    }
}
