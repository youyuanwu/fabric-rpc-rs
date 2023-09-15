// generate server code
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate_internal(service: &prost_build::Service) -> TokenStream {
    let service_ident = quote::format_ident!("{}Service", service.name);
    let server_mod = quote::format_ident!("{}_server", service.name.to_case(Case::Snake));
    let service_router_ident = quote::format_ident!("{}ServiceRouter", service.name);

    let service_name = format!("{}.{}", service.package, service.name);

    let trait_methods = generate_service_trait_methods(service);

    let routing_code = generate_routing_branches(service);
    // print!("{}", routing_code);
    quote! {
      pub mod #server_mod{
        use fabric_rpc_rs::server::{encode_proto, parse_proto, Service};

        // User needs to implement
        #[tonic::async_trait]
        pub trait #service_ident: Send + Sync + 'static {
            #trait_methods
        }

        // Router used for routing
        pub struct #service_router_ident<T: #service_ident> {
            _svc: T,
        }

        impl<T: #service_ident> #service_router_ident<T> {
          pub fn new(svc: T) -> #service_router_ident<T> {
            #service_router_ident { _svc: svc }
          }
      }

        #[tonic::async_trait]
        impl<T: #service_ident> Service for #service_router_ident<T> {
            fn name(&self) -> String {
                String::from(#service_name)
            }

            #[must_use]
            async fn handle_request(
                &self,
                url: String,
                request: &[u8],
            ) -> std::result::Result<Vec<u8>, tonic::Status> {
                match url.as_str() {
                   #routing_code
                    _ => Err(tonic::Status::unimplemented("url not found")),
                }
            }
        }

      }
    }
}

fn generate_service_trait_methods(service: &prost_build::Service) -> TokenStream {
    let mut stream = TokenStream::new();
    for method in &service.methods {
        if method.client_streaming || method.server_streaming {
            // do not support streaming
            continue;
        }
        let ident = format_ident!("{}", method.name);
        let request_type = format_ident!("{}", method.input_type);
        let response_type = format_ident!("{}", method.output_type);
        let method_desc = quote! {
          async fn #ident(request: super::#request_type) -> Result<super::#response_type, tonic::Status>;
        };
        stream.extend(method_desc);
    }
    stream
}

fn generate_routing_branches(service: &prost_build::Service) -> TokenStream {
    let mut stream = TokenStream::new();
    for method in &service.methods {
        if method.client_streaming || method.server_streaming {
            // do not support streaming
            continue;
        }
        let ident = format_ident!("{}", method.name);
        let url = format!("/{}.{}/{}", service.package, service.name, method.name);
        let routing_branch = quote! {
          #url => {
            let req = parse_proto(request)?;
            let resp = T::#ident(req).await?;
            return encode_proto(&resp);
        }
        };
        stream.extend(routing_branch);
    }
    stream
}
