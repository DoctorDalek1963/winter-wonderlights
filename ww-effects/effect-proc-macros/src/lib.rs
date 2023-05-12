//! Provide macros for working with effects in Winter WonderLights.

use proc_macro::TokenStream;

mod derive;
mod generate_lists_and_impls;

/// Derive the [`BaseEffect`](../ww_effects/traits/trait.BaseEffect.html) trait for the given type.
///
/// This derivation assumes that the type has a field `config` of type `<Self as Effect>::Config`
/// and it loads that config from the file and uses `<Self as Default>` for all the other fields.
/// It also assumes that `Self: Effect`.
///
/// See [`Effect`](../ww_effects/traits/trait.Effect.html).
#[proc_macro_derive(BaseEffect)]
pub fn derive_base_effect(input: TokenStream) -> TokenStream {
    derive::derive_base_effect(input)
}

/// Derive the `Sealed` trait for this type.
#[proc_macro_derive(Sealed)]
pub fn derive_sealed(input: TokenStream) -> TokenStream {
    derive::derive_sealed(input)
}

/// Given a list of effect names separated by commas, generate the `EffectNameList` and friends for
/// those effects, as well as all their `impl` blocks.
#[proc_macro]
pub fn generate_lists_and_impls(input: TokenStream) -> TokenStream {
    generate_lists_and_impls::generate_lists_and_impls(input)
}
