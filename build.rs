fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", "./build/_deps/protoc-src/bin/protoc.exe");
    tonic_build::compile_protos("proto/helloworld.proto")?;
    Ok(())
}
