use std::{io, path::Path};

use code_gen::ServiceGenerator;
use prost_build::Config;

mod client;
mod code_gen;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

// code gen builder
pub struct Builder {}

pub fn configure() -> Builder {
    Builder {}
}

impl Builder {
    /// Compile the .proto files and execute code generation.
    pub fn compile(
        self,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        //self.compile_with_config(Config::new(), protos, includes)
        let mut config = Config::new();
        // add generator
        config.service_generator(self.service_generator());
        config.compile_protos(protos, includes)?;
        Ok(())
    }

    // turn builder into generator
    pub fn service_generator(self) -> Box<dyn prost_build::ServiceGenerator> {
        Box::new(ServiceGenerator::new())
    }
}

pub fn compile_protos(proto: impl AsRef<Path>) -> io::Result<()> {
    let proto_path: &Path = proto.as_ref();

    // directory the main .proto file resides in
    let proto_dir = proto_path
        .parent()
        .expect("proto file should reside in a directory");

    self::configure().compile(&[proto_path], &[proto_dir])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
