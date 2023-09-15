# Benchmark Fabric RPC Client

The goal of the `benches` directory is to have a set of benchmarks to test the RPC functionality.
Currently there is `fabric_transport_client` that runs under the crate `criterion` that allows to test the RPC client.

## Run benchmark

In order to run the benchmark follow these steps:

1. Start the RPC `fabric_server` in one terminal for instance (see `fabric_server.rs`) using the following commands:
    ```sh
    cargo build --bin fabric_server
    cargo run --bin fabric_server
    ```
    This server listens to connections on `localhost:12345`
    ```sh
    cargo run --bin fabric_server
    Compiling fabric-rpc-rs v0.1.0 (..\fabric-rpc-rs)
    Finished dev [unoptimized + debuginfo] target(s) in 1.55s
     Running `target\debug\fabric_server.exe`
    Server. Creating...
    Server listening at "localhost:12345+/"
    ```

1. Run `fabric_transport_client` benchmark on a second terminal
    ```sh
    cargo bench
    ```

    ```sh
    Compiling fabric-rpc-rs v0.1.0 (..\fabric-rpc-rs)
    Finished bench [optimized] target(s) in 2.83s                                                 
     Running unittests src\lib.rs (target\release\deps\fabric_rpc_rs-9382d0e7dcb7ad43.exe)        

    ....
    test result: ok. 0 passed; 0 failed; 8 ignored; 0 measured; 0 filtered out; finished in 0.00s     

    Running src/benches/fabric_rpc_benchmark.rs (target\release\deps\fabric_rpc_benchmark-92112ec8c8c4abd3.exe)
    Gnuplot not found, using plotters backend
    Benchmarking fabric_transport_client: Warming up for 3.0000 s
    Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 7.4s, enable flat sampling, or reduce sample count to 50.
    Benchmarking fabric_transport_client: Collecting 100 samples in estimated 7.3679 s (5050 iterationfabric_transport_client time:   [1.4458 ms 1.4539 ms 1.4617 ms]
                        change: [-4.4399% -3.6932% -2.8653%] (p = 0.00 < 0.05)
                        Performance has improved.
    Found 6 outliers among 100 measurements (6.00%)
    4 (4.00%) high mild
    2 (2.00%) high severe
    ```

## References

[criterion docs](https://docs.rs/criterion/latest/criterion)

[tokio docs](https://docs.rs/tokio/latest/tokio/)