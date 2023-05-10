use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive the [`BaseEffect`](../ww_effects/traits/trait.BaseEffect.html) trait for the given type.
///
/// This derivation assumes that the type has a field `config` of type `<Self as Effect>::Config`
/// and it loads that config from the file and uses `<Self as Default>` for all the other fields.
/// It also assumes that `Self: Effect`.
///
/// See [`Effect`](../ww_effects/traits/trait.Effect.html).
#[proc_macro_derive(BaseEffect)]
pub fn derive_base_effect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;

    quote! {
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
