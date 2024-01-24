//! Provide macros for working with effects in `Winter WonderLights`.

#![feature(lint_reasons)]
#![allow(
    clippy::expect_used,
    reason = "we can't used expect_or_log here because this is a proc-macro crate"
)]

use proc_macro::TokenStream;

mod derive;
mod end_loop;
mod generate_lists_and_impls;

/// Derive the [`BaseEffect`](../ww_effects/traits/trait.BaseEffect.html) trait for the given type.
///
/// This derivation assumes that the type has a field `config` of type `<Self as
/// BaseEffect>::Config` (see [`BaseEffect::Config`](../ww_effects/traits/trait.BaseEffect.html))
/// and it loads that config from the file and uses `<Self as Default>` for all the other fields.
///
/// The type must also implement [`Effect`](../ww_effects/traits/trait.Effect.html).
#[proc_macro_derive(BaseEffect)]
pub fn derive_base_effect(input: TokenStream) -> TokenStream {
    derive::derive_base_effect(input)
}

/// Derive the [`BaseEffectConfig`](../ww_effects/traits/trait.BaseEffectConfig.html) trait for the
/// given type.
///
/// The type must also implement [`EffectConfig`](../ww_effects/traits/trait.EffectConfig.html).
#[proc_macro_derive(BaseEffectConfig)]
pub fn derive_base_effect_config(input: TokenStream) -> TokenStream {
    derive::derive_base_effect_config(input)
}

/// Given a list of effect names separated by commas, generate the `EffectNameList` and friends for
/// those effects, as well as all their `impl` blocks.
#[proc_macro]
pub fn generate_lists_and_impls(input: TokenStream) -> TokenStream {
    generate_lists_and_impls::generate_lists_and_impls(input)
}

/// End a loop after 100 iterations in test or benchmark builds.
#[proc_macro_attribute]
pub fn end_loop_in_test_or_bench(_args: TokenStream, input: TokenStream) -> TokenStream {
    end_loop::end_loop_in_test_or_bench(input.into()).into()
}
