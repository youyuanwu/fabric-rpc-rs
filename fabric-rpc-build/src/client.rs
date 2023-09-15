use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use prost_build::Method;
use quote::{format_ident, quote};

// client code
pub fn generate_internal(service: &prost_build::Service) -> TokenStream {
    let service_ident = quote::format_ident!("{}Client", service.name);
    let client_mod = quote::format_ident!("{}_client", service.name.to_case(Case::Snake));

    let methods = generate_methods(service);
    // println!("{}",methods);
    quote! {
        pub mod #client_mod {
            use fabric_rpc_rs::client::Client2;
            use windows::core::{Error, HSTRING};

            pub struct #service_ident{
                c: Client2
            }

            impl #service_ident {
                pub async fn connect(addr: HSTRING) -> Result<#service_ident, Error> {
                    let c = Client2::connect(addr).await?;
                    Ok(#service_ident { c })
                }
                #methods
            }

        }
    }
}

pub fn generate_methods(service: &prost_build::Service) -> TokenStream {
    let mut stream = TokenStream::new();

    for method in &service.methods {
        if method.client_streaming || method.server_streaming {
            // do not support streaming
            continue;
        }
        stream.extend(generate_unary(service, method));
    }
    stream
}

fn generate_unary(service: &prost_build::Service, method: &Method) -> TokenStream {
    let ident = format_ident!("{}", method.name);
    let request_type = format_ident!("{}", method.input_type);
    let response_type = format_ident!("{}", method.output_type);
    let url = format!("/{}.{}/{}", service.package, service.name, method.name);
    quote! {
        pub async fn #ident (&self,
            timoutmilliseconds: u32,
            request: super::#request_type,
        ) -> Result<super::#response_type, tonic::Status> {
            let url = String::from(#url);
            self.c.request(url, &request, timoutmilliseconds).await
        }
    }
}
