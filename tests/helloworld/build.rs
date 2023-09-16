fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", "../../build/_deps/protoc-src/bin/protoc.exe");
    // generate fabric-rpc example code
    fabric_rpc_build::compile_protos("../../proto/fabrichello.proto")?;

    fabric_rpc_build::compile_protos("../../proto/todolist.proto")?;
    Ok(())
}
