use proc_macro2::TokenStream;

use crate::client;
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
        let code = builder.generate_client(&service);
        buf.push_str(code.to_string().as_str());
    }
}

pub struct CodeGenBuilder {}

impl CodeGenBuilder {
    pub fn generate_client(&self, service: &prost_build::Service) -> TokenStream {
        client::generate_internal(service)
    }
}
