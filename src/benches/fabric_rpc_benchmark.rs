use criterion::{criterion_group, criterion_main, Criterion};

use fabric_base::{
    FabricCommon::FabricTransport::FABRIC_TRANSPORT_SETTINGS, FABRIC_SECURITY_CREDENTIALS,
    FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
};
use fabric_rpc_rs::{
    client_tr::ClientTransport,
    sys::{Message, MessageViewer},
};

use tokio::runtime::Runtime;
use windows::core::HSTRING;

async fn fabric_transport_client() {
    const TIMEOUT_MILLIS: u32 = 100000;

    let creds = FABRIC_SECURITY_CREDENTIALS {
        Kind: FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
        ..Default::default()
    };
    let settings = FABRIC_TRANSPORT_SETTINGS {
        KeepAliveTimeoutInSeconds: 10,
        MaxConcurrentCalls: 10,
        MaxMessageSize: 10,
        MaxQueueSize: 10,
        OperationTimeoutInSeconds: 10,
        SecurityCredentials: &creds,
        ..Default::default()
    };

    let connectionaddress = HSTRING::from("localhost:12345+/");
    let mut client = ClientTransport::new(&settings, &connectionaddress).unwrap();
    match client.open(TIMEOUT_MILLIS).await {
        Err(e) => {
            eprintln!("Client unable to connect: {:?}", e);
            return;
        }
        _ => {}
    }

    // This wait is optional in prod
    client.connect().await;

    // send request
    {
        let header = String::from("myheader");
        let body = String::from("mybody");
        let msg = Message::create(header.clone().into_bytes(), body.clone().into_bytes());
        let reply = client.request(TIMEOUT_MILLIS, &msg).await.unwrap();
        let replyvw = MessageViewer::new(reply);

        let header_ret = replyvw.get_header();
        assert_eq!(header_ret, String::from("hello: myheader").as_bytes());

        let body_ret = replyvw.get_body();
        assert_eq!(body_ret, String::from("hello: mybody").as_bytes());
    }

    // close client
    client.close(TIMEOUT_MILLIS).await.unwrap();
}

fn criterion_fabric_transport(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("fabric_transport_client", move |b| {
        b.to_async(&rt).iter(|| async move {
            fabric_transport_client().await;
        });
    });
}

criterion_group!(benches, criterion_fabric_transport);
criterion_main!(benches);
