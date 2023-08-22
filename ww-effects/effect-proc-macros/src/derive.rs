//! Handle deriving traits.

use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

/// Derive the `BaseEffect` trait.
pub fn derive_base_effect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;
    let sealed_impl = create_sealed_impl(&struct_name);
    let config_ident = format_ident!("{}Config", struct_name);

    quote! {
        #sealed_impl

        impl crate::traits::BaseEffect for #struct_name {
            type Config = #config_ident;

            fn effect_name() -> &'static str {
                stringify!(#struct_name)
            }

            fn save_to_file(&self) {
                self.config.save_to_file(&<Self as crate::traits::Effect>::config_filename());
            }

            fn from_file() -> Self {
                Self::from_config(
                    Self::Config::from_file(
                        &<Self as crate::traits::Effect>::config_filename()
                    )
                )
            }
        }

        impl ::std::default::Default for #struct_name {
            fn default() -> Self {
                Self::from_config(<Self as BaseEffect>::Config::default())
            }
        }
    }
    .into()
}

/// Derive the `BaseEffectConfig` trait.
pub fn derive_base_effect_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;
    let sealed_impl = create_sealed_impl(&struct_name);
    let heading = Literal::string(&format!(
        "{} config",
        &struct_name.to_string().replace("Config", "")
    ));

    quote! {
        #sealed_impl

        impl crate::traits::BaseEffectConfig for #struct_name {
            fn render_full_options_gui(&mut self, ctx: &::egui::Context, ui: &mut ::egui::Ui) -> bool {
                ui.label(egui::RichText::new(#heading).heading());
                ui.add_space(crate::effects::prelude::UI_SPACING);

                let mut config_changed = false;

                config_changed |= self.render_options_gui(ctx, ui);

                ui.add_space(crate::effects::prelude::UI_SPACING);

                if ui.button("Reset to defaults").clicked() {
                    *self = Self::default();
                    config_changed = true;
                }

                config_changed
            }
        }
    }
    .into()
}

/// Create an implementation of `Sealed` for a type with the given name.
fn create_sealed_impl(name: &Ident2) -> TokenStream2 {
    quote! {
        impl crate::traits::private::Sealed for #name {}
    }
}
