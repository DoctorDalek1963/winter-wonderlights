//! Handle the [`generate_lists_and_impls`] macro.

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{Error, Result};

/// Generate the name and dispatch lists as well as their implementations.
pub fn generate_lists_and_impls(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match generate_lists_and_impls2(input.into()) {
        Ok(stream) => stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Generate the name and dispatch lists as well as their implementations.
fn generate_lists_and_impls2(input: TokenStream) -> Result<TokenStream> {
    let effect_names = get_effect_names(input)?;
    let config_names: Vec<Ident> = effect_names
        .iter()
        .map(|ident| format_ident!("{ident}Config"))
        .collect();

    let name_lists = create_name_lists(&effect_names, &config_names);
    let dispatch_lists = create_dispatch_lists(&effect_names, &config_names);
    let impls = impl_lists(&effect_names, &config_names);
    let from_impls = impl_from_lists(&effect_names);

    Ok(quote! {
        #[cfg(feature = "config-trait")]
        use crate::traits::EffectConfig;

        #[cfg(feature = "effect-trait")]
        use crate::traits::{BaseEffect, Effect};

        #[cfg(feature = "config-impls")]
        use crate::effects::configs::*;

        #[cfg(feature = "effect-impls")]
        use crate::effects::effects::*;

        #name_lists
        #dispatch_lists
        #impls
        #from_impls
    })
}

/// Get a list of the names of the effects from the input.
fn get_effect_names(input: TokenStream) -> Result<Vec<Ident>> {
    let mut effect_names: Vec<Ident> = Vec::new();
    let mut last_elem_was_ident = false;

    for tt in input {
        match tt {
            TokenTree::Ident(ident) => {
                if last_elem_was_ident {
                    return Err(Error::new(
                        ident.span(),
                        "Effect identifiers must be separated with commas",
                    ));
                }

                effect_names.push(ident);
                last_elem_was_ident = true;
            }
            TokenTree::Punct(punct) => {
                if punct.as_char() != ',' {
                    return Err(Error::new(
                        punct.span(),
                        "Effect identifiers must be separated with commas",
                    ));
                }

                last_elem_was_ident = false;
            }
            _ => return Err(syn::Error::new(
                Span::call_site(),
                "`generate_lists_and_impls!()` takes a list of comma-separated effect identifiers",
            )),
        }
    }

    Ok(effect_names)
}

/// Create the `*NameList` enums for the effects and configs.
fn create_name_lists(effect_names: &[Ident], config_names: &[Ident]) -> TokenStream {
    let effect_items: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let doc_comment: TokenStream = format!("/// See [`{ident}`]")
                .parse()
                .expect("Parsing this doc comment as a TokenStream should never fail");
            quote! {
                #doc_comment
                #ident
            }
        })
        .collect();

    let config_items: Vec<_> = config_names
        .iter()
        .map(|ident| {
            let doc_comment: TokenStream = format!("/// See [`{ident}`]")
                .parse()
                .expect("Parsing this doc comment as a TokenStream should never fail");
            quote! {
                #doc_comment
                #ident
            }
        })
        .collect();

    quote! {
        /// This enum has a variant for each effect, but only the names. If the `effect-impls` feature is
        /// enabled, then you can call certain methods on this enum to get things like the like the
        /// [`next_frame`](Effect::next_frame) method.
        ///
        /// If an effect is not accessible via this enum, then it should not be used.
        ///
        /// See `EffectDispatchList` for wrappers of instances of effects, or call `.into()` to read the
        /// effect from its file.
        #[derive(Clone, Copy, Debug, Eq, PartialEq, ::strum::EnumIter, ::serde::Serialize, ::serde::Deserialize)]
        pub enum EffectNameList {
            #( #effect_items ),*
        }

        /// This enum has a variant for each effect config, but only the names. If the `config-impls`
        /// feature is enabled, then you can create an [`EffectConfigDispatchList`] by calling `.into()`.
        #[derive(Clone, Copy, Debug, Eq, PartialEq, ::strum::EnumIter, ::serde::Serialize, ::serde::Deserialize)]
        pub enum EffectConfigNameList {
            #( #config_items ),*
        }

    }
}

/// Create the `*DispatchList` enums for the effects and configs.
fn create_dispatch_lists(effect_names: &[Ident], config_names: &[Ident]) -> TokenStream {
    let effect_items: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let doc_comment: TokenStream = format!("/// See [`{ident}`]")
                .parse()
                .expect("Parsing this doc comment as a TokenStream should never fail");
            quote! {
                #doc_comment
                #ident ( #ident )
            }
        })
        .collect();

    let config_items: Vec<_> = config_names
        .iter()
        .map(|ident| {
            let doc_comment: TokenStream = format!("/// See [`{ident}`]")
                .parse()
                .expect("Parsing this doc comment as a TokenStream should never fail");
            quote! {
                #doc_comment
                #ident ( #ident )
            }
        })
        .collect();

    quote! {
        /// This enum has a variant to wrap an instance of every effect. You can call any method
        /// from the [`Effect`] trait on a variant of this enum.
        #[cfg(feature = "effect-impls")]
        #[derive(Clone, Debug, PartialEq)]
        pub enum EffectDispatchList {
            #( #effect_items ),*
        }

        /// This enum has a variant to wrap an instance of every effect config. You can call most
        /// methods from the [`EffectConfig`] trait on a variant of this enum.
        #[cfg(feature = "config-impls")]
        #[derive(Clone, Debug, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        pub enum EffectConfigDispatchList {
            #( #config_items ),*
        }

    }
}

/// Implement methods on the lists.
fn impl_lists(effect_names: &[Ident], config_names: &[Ident]) -> TokenStream {
    let effect_name_list_effect_names: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let string = ident.to_string();
            quote! {
                EffectNameList:: #ident => #string
            }
        })
        .collect();

    let effect_name_list_configs_from_file: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let config_ident = format_ident!("{ident}Config");
            let ident_name = ident.to_string();
            quote! {
                EffectNameList:: #ident => {
                    EffectConfigDispatchList:: #config_ident (
                        #config_ident ::from_file(&crate::traits::get_config_filename( #ident_name ))
                    )
                }
            }
        })
        .collect();

    let effect_name_list_config_names: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let config = format_ident!("{ident}Config");
            quote! {
                EffectNameList:: #ident => EffectConfigNameList:: #config
            }
        })
        .collect();

    let effect_name_list_from_file: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            quote! {
                EffectNameList:: #ident => EffectDispatchList:: #ident (#ident ::from_file())
            }
        })
        .collect();

    let effect_name_list_default_dispatch: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            quote! {
                EffectNameList:: #ident => EffectDispatchList:: #ident ( #ident ::default())
            }
        })
        .collect();

    let effect_dispatch_list_effect_names: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let string = ident.to_string();
            quote! {
                EffectDispatchList:: #ident (_) => #string
            }
        })
        .collect();

    let effect_dispatch_list_next_frame: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let config_ident = format_ident!("{ident}Config");
            quote! {
                (
                    EffectDispatchList:: #ident (effect),
                    EffectConfigDispatchList:: #config_ident (config)
                ) => effect.next_frame(config)
            }
        })
        .collect();

    let effect_dispatch_list_configs_from_file: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let config_ident = format_ident!("{ident}Config");
            quote! {
                EffectDispatchList:: #ident (_) => EffectConfigDispatchList:: #config_ident (#ident ::config_from_file())
            }
        })
        .collect();

    let effect_dispatch_list_loops_to_test: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            quote! {
                EffectDispatchList:: #ident (_) => #ident ::loops_to_test()
            }
        })
        .collect();

    let config_name_list_configs_from_file: Vec<_> = config_names
        .iter()
        .map(|ident| {
            let effect_name = ident.to_string().replace("Config", "");
            quote! {
                EffectConfigNameList:: #ident => {
                    EffectConfigDispatchList:: #ident (
                        #ident ::from_file(&crate::traits::get_config_filename( #effect_name ))
                    )
                }
            }
        })
        .collect();

    let config_name_list_effect_names: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let string = ident.to_string();
            let config = format_ident!("{ident}Config");
            quote! {
                EffectConfigNameList:: #config => #string
            }
        })
        .collect();

    let config_name_list_default_dispatch: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let config = format_ident!("{ident}Config");
            quote!{
                EffectConfigNameList:: #config => EffectConfigDispatchList:: #config ( #config ::default())
            }
        })
        .collect();

    let config_dispatch_list_render_full_options_guis: Vec<_> = config_names
        .iter()
        .map(|ident| {
            quote! {
                EffectConfigDispatchList:: #ident (config) => config.render_full_options_gui(ctx, ui)
            }
        })
        .collect();

    let config_dispatch_list_save_to_file: Vec<_> = config_names
        .iter()
        .map(|ident| {
            quote! {
                EffectConfigDispatchList:: #ident (config) => crate::save_effect_config_to_file(filename, config)
            }
        })
        .collect();

    let config_dispatch_list_effect_names: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let string = ident.to_string();
            let config = format_ident!("{ident}Config");
            quote! {
                EffectConfigDispatchList:: #config (_) => #string
            }
        })
        .collect();

    quote! {
        impl EffectNameList {
            /// Get the name of the effect as a `&str`.
            pub fn effect_name(&self) -> &'static str {
                match self {
                    #( #effect_name_list_effect_names ),*
                }
            }

            /// Get the name of the associated config.
            pub fn config_name(&self) -> EffectConfigNameList {
                match self {
                    #( #effect_name_list_config_names ),*
                }
            }

            /// Get the config for this effect, loaded from its file.
            #[cfg(feature = "config-impls")]
            pub fn config_from_file(&self) -> EffectConfigDispatchList {
                match self {
                    #( #effect_name_list_configs_from_file ),*
                }
            }

            /// Load this effect from the config given in its file.
            #[cfg(feature = "effect-impls")]
            pub fn from_file(&self) -> EffectDispatchList {
                match self {
                    #( #effect_name_list_from_file ),*
                }
            }

            /// Get the default [`EffectDispatchList`] for this effect name.
            #[cfg(feature = "effect-impls")]
            pub fn default_dispatch(&self) -> EffectDispatchList {
                match self {
                    #( #effect_name_list_default_dispatch ),*
                }
            }
        }

        #[cfg(feature = "effect-impls")]
        impl EffectDispatchList {
            /// Get the name of the effect as a `&str`.
            pub fn effect_name(&self) -> &'static str {
                match self {
                    #( #effect_dispatch_list_effect_names ),*
                }
            }

            /// Get the next frame for this effect. See [`Effect::next_frame`].
            pub fn next_frame(
                &mut self,
                config: &EffectConfigDispatchList,
            ) -> Option<(::ww_frame::FrameType, ::std::time::Duration)> {
                match (self, config) {
                    #( #effect_dispatch_list_next_frame ),*,
                    (effect, config) => {
                        ::tracing::warn!(
                            "Cannot get the next frame for effect {} with config for {}, so terminating effect",
                            effect.effect_name(),
                            config.effect_name()
                        );
                        None
                    }
                }
            }

            /// Get the config for this effect, loaded from its file.
            pub fn config_from_file(&self) -> EffectConfigDispatchList {
                match self {
                    #( #effect_dispatch_list_configs_from_file ),*
                }
            }

            /// Get the number of loops to test in a benchmark.
            #[cfg(feature = "bench")]
            pub fn loops_to_test(&self) -> Option<::std::num::NonZeroU16> {
                match self {
                    #( #effect_dispatch_list_loops_to_test ),*
                }
            }
        }

        impl EffectConfigNameList {
            /// Load the config from its file.
            #[cfg(feature = "config-impls")]
            pub fn config_from_file(&self) -> EffectConfigDispatchList {
                match self {
                    #( #config_name_list_configs_from_file ),*
                }
            }

            /// Get the name of the corresponding effect.
            pub fn effect_name(&self) -> &'static str {
                match self {
                    #( #config_name_list_effect_names ),*
                }
            }

            /// Get the default [`EffectConfigDispatchList`] for this effect name.
            #[cfg(feature = "config-impls")]
            pub fn default_dispatch(&self) -> EffectConfigDispatchList {
                match self {
                    #( #config_name_list_default_dispatch ),*
                }
            }
        }

        #[cfg(feature = "config-impls")]
        impl EffectConfigDispatchList {
            /// Render the options GUI for the config.
            pub fn render_full_options_gui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
                use crate::traits::BaseEffectConfig;

                match self {
                    #( #config_dispatch_list_render_full_options_guis ),*
                }
            }

            /// Save the config to its file.
            pub fn save_to_file(&self, filename: &str) {
                match self {
                    #( #config_dispatch_list_save_to_file ),*
                }
            }

            /// Get the name of the corresponding effect.
            pub fn effect_name(&self) -> &'static str {
                match self {
                    #( #config_dispatch_list_effect_names ),*
                }
            }
        }
    }
}

/// Implement the `From<T>` trait to convert between the lists.
fn impl_from_lists(effect_names: &[Ident]) -> TokenStream {
    let effect_name_list_to_dispatch: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            quote! {
                EffectNameList:: #ident => EffectDispatchList:: #ident ( #ident ::from_file())
            }
        })
        .collect();

    let effect_dispatch_list_to_name: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            quote! {
                EffectDispatchList:: #ident (_) => EffectNameList:: #ident
            }
        })
        .collect();

    let effect_name_list_to_config_name: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let config_name = format_ident!("{ident}Config");
            quote! {
                EffectNameList:: #ident => EffectConfigNameList:: #config_name
            }
        })
        .collect();

    let config_name_list_to_effect_name: Vec<_> = effect_names
        .iter()
        .map(|ident| {
            let config_name = format_ident!("{ident}Config");
            quote! {
                EffectConfigNameList:: #config_name => EffectNameList:: #ident
            }
        })
        .collect();

    quote! {
        #[cfg(feature = "config-impls")]
        impl From<EffectConfigNameList> for EffectConfigDispatchList {
            fn from(value: EffectConfigNameList) -> Self {
                value.config_from_file()
            }
        }

        #[cfg(feature = "effect-impls")]
        impl From<EffectNameList> for EffectDispatchList {
            fn from(value: EffectNameList) -> Self {
                match value {
                    #( #effect_name_list_to_dispatch ),*
                }
            }
        }

        #[cfg(feature = "effect-impls")]
        impl From<&EffectNameList> for EffectDispatchList {
            fn from(value: &EffectNameList) -> Self {
                match value {
                    #( #effect_name_list_to_dispatch ),*
                }
            }
        }

        #[cfg(feature = "effect-impls")]
        impl From<EffectDispatchList> for EffectNameList {
            fn from(value: EffectDispatchList) -> Self {
                match value {
                    #( #effect_dispatch_list_to_name ),*
                }
            }
        }

        #[cfg(feature = "effect-impls")]
        impl From<&EffectDispatchList> for EffectNameList {
            fn from(value: &EffectDispatchList) -> Self {
                match value {
                    #( #effect_dispatch_list_to_name ),*
                }
            }
        }

        impl From<EffectNameList> for EffectConfigNameList {
            fn from(value: EffectNameList) -> Self {
                match value {
                    #( #effect_name_list_to_config_name ),*
                }
            }
        }

        impl From<EffectConfigNameList> for EffectNameList {
            fn from(value: EffectConfigNameList) -> Self {
                match value {
                    #( #config_name_list_to_effect_name ),*
                }
            }
        }
    }
}
