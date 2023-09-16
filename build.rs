fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", "./build/_deps/protoc-src/bin/protoc.exe");
    // generate helloworld for grpc
    tonic_build::compile_protos("proto/helloworld.proto")?;

    // generate fabric-rpc header
    prost_build::compile_protos(&["proto/fabricrpc.proto"], &["proto/"])?;

    // generate fabrichello for benchmark
    fabric_rpc_build::compile_protos("proto/fabrichello.proto")?;

    Ok(())
}
