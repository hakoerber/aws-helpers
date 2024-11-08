#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "panics and expects are fine for proc macros"
)]

use proc_macro::TokenStream;

mod tag;
mod tags;

#[proc_macro_attribute]
#[expect(non_snake_case, reason = "attribute proc macros should be capitalized")]
pub fn Tags(attr: TokenStream, item: TokenStream) -> TokenStream {
    tags::transform(attr, item)
}

#[proc_macro_derive(Tag, attributes(tag))]
pub fn tag(input: TokenStream) -> TokenStream {
    tag::transform(input)
}
