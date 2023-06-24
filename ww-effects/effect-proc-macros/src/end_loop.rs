//! This module provides the [`end_loop_in_test_or_bench`] attribute macro, intended to be used
//! like so:
//!
//! ```ignore
//! impl Effect for MyEffect {
//!     async fn run(self, &mut dyn Driver) {
//!         // Setup code here
//!
//!         #[end_loop_in_test_or_bench]
//!         loop {
//!             // Effect code here
//!         }
//!     }
//! }
//! ```
//!
//! The attribute will add a counter in test or benchmark builds which will end the loop after 100
//! iterations.

use proc_macro2::{Delimiter, Ident, TokenStream, TokenTree};
use quote::quote;
use syn::{spanned::Spanned, Error};

/// End a loop after 100 iterations in test or benchmark builds.
pub fn end_loop_in_test_or_bench(input: TokenStream) -> TokenStream {
    let input_span = input.span();

    let tokens: Vec<_> = input.into_iter().collect();

    match tokens.first() {
        Some(TokenTree::Ident(ident))
            if {
                let span = ident.span();
                ident == &Ident::new("loop", span)
            } =>
        {
            let body: TokenStream = match tokens.get(1) {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                    group.stream()
                }
                _ => {
                    return Error::new(input_span, "No group after `loop` keyword")
                        .to_compile_error()
                }
            };

            quote! {
                {
                    #[cfg(any(test, feature = "bench"))]
                    let mut end_loop_in_test_or_bench_counter = 0u8;

                    loop {
                        #body

                        #[cfg(any(test, feature = "bench"))]
                        {
                            end_loop_in_test_or_bench_counter += 1;
                            if end_loop_in_test_or_bench_counter > 100 {
                                break;
                            }
                        }
                    }
                }
            }
        }
        _ => Error::new(
            input_span,
            "#[end_loop_in_test_or_bench] can only be used on `loop {}` constructs",
        )
        .to_compile_error(),
    }
}
