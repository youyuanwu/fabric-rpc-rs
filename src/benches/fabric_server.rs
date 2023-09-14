use fabric_rpc_rs::{
    server_tr::ServerTransport,
    sys::{Message, MessageViewer},
};
use service_fabric_rs::{
    FabricCommon::FabricTransport::{FABRIC_TRANSPORT_LISTEN_ADDRESS, FABRIC_TRANSPORT_SETTINGS},
    FABRIC_SECURITY_CREDENTIALS, FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
};
use windows::core::{HSTRING, PCWSTR};

async fn start_transport_server() {
    println!("Server. Creating...");
    let (stoptx, mut stoprx) = tokio::sync::oneshot::channel::<()>();

    let mut listener: ServerTransport;
    {
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

        // create server
        let host = HSTRING::from("localhost");
        let path = HSTRING::from("/");
        let serveraddr = FABRIC_TRANSPORT_LISTEN_ADDRESS {
            IPAddressOrFQDN: PCWSTR::from(&host),
            Port: 12345,
            Path: PCWSTR::from(&path),
        };
        listener = ServerTransport::new(&settings, &serveraddr).unwrap();
    }
    let listen_addr = listener.open().await.unwrap();

    let connectionaddress = HSTRING::from("localhost:12345+/");
    assert_eq!(listen_addr, connectionaddress);
    println!("Server listening at {:?}", connectionaddress);

    loop {
        let mut conn;
        tokio::select! {
            _ = (&mut stoprx) => { break;},
            x = listener.async_accept() => {
                conn = x;
            }
        }
        tokio::spawn(async move {
            let mut req = conn.async_accept().await;

            let msg = req.get_request_msg();
            let vw = MessageViewer::new(msg.clone());

            let header = vw.get_header();
            let body = vw.get_body();

            let hello = String::from("hello: ").into_bytes();
            let mut reply_header = hello.clone();
            reply_header.extend(header);
            let mut reply_body = hello;
            reply_body.extend(body);

            let reply = Message::create(reply_header, reply_body);
            req.complete(reply);
        })
        .await
        .unwrap();
    }
    listener.close().await.unwrap();
    // stop server
    stoptx.send(()).unwrap();
}

#[tokio::main]
async fn main() {
    start_transport_server().await;
}
