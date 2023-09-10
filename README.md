# fabric-rpc-rs
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://raw.githubusercontent.com/youyuanwu/fabric-rpc-rs/main/LICENSE)

RPC framework based on FabricTransport in Service Fabric. Rust support.
See C++ counterpart in [fabric-rpc](https://github.com/youyuanwu/fabric-rpc).

# Dependencies
Required:
* service fabric runtime installation. See [get-started](https://learn.microsoft.com/en-us/azure/service-fabric/service-fabric-get-started)

Cargo auto managed:
* [fabric-metadata](https://github.com/youyuanwu/fabric-metadata). Service fabric support libraries.
* [service-fabric-rs](https://github.com/youyuanwu/service-fabric-rs). Service fabric rust bindings.

# Build and Test
```ps1
# build code
cargo build

# test code
cargo test
```

# License
MIT License