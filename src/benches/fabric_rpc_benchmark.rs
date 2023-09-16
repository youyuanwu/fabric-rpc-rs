use criterion::{criterion_group, criterion_main, Criterion};

use fabric_rpc_rs::server::Server;

use tokio::{runtime::Runtime, sync::oneshot::Receiver};
use windows::core::HSTRING;

#[allow(non_snake_case)]
pub mod gen {
    tonic::include_proto!("fabrichello"); // The string specified here must match the proto package name
}

pub struct HelloSvcImpl {}

#[tonic::async_trait]
impl gen::fabric_hello_server::FabricHelloService for HelloSvcImpl {
    async fn say_hello(
        &self,
        request: gen::FabricRequest,
    ) -> Result<gen::FabricResponse, tonic::Status> {
        let name = request.fabric_name;
        let mut msg_reply = String::from("Hello: ");
        msg_reply.push_str(name.as_str());
        let reply = gen::FabricResponse {
            fabric_message: msg_reply,
        };
        Ok(reply)
    }
}

fn run_server(stoprx: Receiver<()>) {
    tokio::spawn(async move {
        // make server run
        let hello_svc = HelloSvcImpl {};

        let mut svr = Server::default();
        svr.add_service(gen::fabric_hello_server::FabricHelloServiceRouter::new(
            hello_svc,
        ));
        svr.serve_with_shutdown(12345, async {
            stoprx.await.ok();
            println!("Graceful shutdown complete")
        })
        .await;
    });
}

async fn make_request(c: &gen::fabric_hello_client::FabricHelloClient) {
    let request = gen::FabricRequest {
        fabric_name: String::from("myname"),
    };
    let resp = c.say_hello(10000, request).await.unwrap();

    assert_eq!("Hello: myname", resp.fabric_message);
}

fn criterion_fabric_transport(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (stoptx, stoprx) = tokio::sync::oneshot::channel::<()>();
    // run server
    rt.spawn(async move {
        run_server(stoprx);
    });
    // create client sync
    let client = rt.block_on(async {
        let connectionaddress = HSTRING::from("localhost:12345+/");
        gen::fabric_hello_client::FabricHelloClient::connect(connectionaddress)
            .await
            .unwrap()
    });

    for _ in 1..3 {
        // TODO: spawning does not work.
        rt.block_on(async {
            make_request(&client).await;
        });
    }
    // c.bench_function("fabric_transport_client", move |b| {
    //     let client_ref = &client;
    //     b.to_async(&rt).iter(|| async move {
    //         make_request(client_ref).await;
    //         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //     });
    // });
    stoptx.send(()).unwrap();
}

criterion_group!(benches, criterion_fabric_transport);
criterion_main!(benches);
