use proc_macro2::TokenStream;
use quote::quote;

// client code
pub fn generate_internal(_service: &prost_build::Service) -> TokenStream {
    quote! {
    // hello test
    pub struct MyClientTest{}
    }
}
