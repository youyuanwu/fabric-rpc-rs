use proc_macro2::TokenStream;

use crate::{client, server};
//use prost_build::Service;

pub struct ServiceGenerator {
    //builder: Builder
}

impl ServiceGenerator {
    pub fn new() -> Self {
        ServiceGenerator {
          // builder,
          // clients: TokenStream::default(),
          // servers: TokenStream::default(),
      }
    }
}

impl prost_build::ServiceGenerator for ServiceGenerator {
    fn generate(&mut self, service: prost_build::Service, buf: &mut String) {
        let builder = CodeGenBuilder {};
        let client_code = builder.generate_client(&service);
        buf.push_str(client_code.to_string().as_str());
        let server_code = builder.generate_server(&service);
        buf.push_str(server_code.to_string().as_str());
    }
}

struct CodeGenBuilder {}

impl CodeGenBuilder {
    pub fn generate_client(&self, service: &prost_build::Service) -> TokenStream {
        client::generate_internal(service)
    }

    pub fn generate_server(&self, service: &prost_build::Service) -> TokenStream {
        server::generate_internal(service)
    }
}
