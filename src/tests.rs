use service_fabric_rs::{
    FabricCommon::FabricTransport::{FABRIC_TRANSPORT_LISTEN_ADDRESS, FABRIC_TRANSPORT_SETTINGS},
    FABRIC_E_CONNECTION_CLOSED_BY_REMOTE_END, FABRIC_SECURITY_CREDENTIALS,
    FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
};
use windows::core::{HRESULT, HSTRING, PCWSTR};

use crate::{
    client_tr::ClientTransport,
    server_tr::ServerTransport,
    sys::{Message, MessageViewer},
};

#[tokio::test]
async fn fabric_transport() {
    let mut creds = FABRIC_SECURITY_CREDENTIALS::default();
    creds.Kind = FABRIC_SECURITY_CREDENTIAL_KIND_NONE;
    let mut settings = FABRIC_TRANSPORT_SETTINGS::default();
    settings.KeepAliveTimeoutInSeconds = 10;
    settings.MaxConcurrentCalls = 10;
    settings.MaxMessageSize = 10;
    settings.MaxQueueSize = 10;
    settings.OperationTimeoutInSeconds = 10;
    settings.SecurityCredentials = &creds;

    // create server
    let mut serveraddr = FABRIC_TRANSPORT_LISTEN_ADDRESS::default();
    let host = HSTRING::from("localhost");
    let path = HSTRING::from("/");
    serveraddr.IPAddressOrFQDN = PCWSTR::from(&host);
    serveraddr.Port = 12345;
    serveraddr.Path = PCWSTR::from(&path);

    let (stoptx, mut stoprx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut listener: ServerTransport;
        {
            let mut creds = FABRIC_SECURITY_CREDENTIALS::default();
            creds.Kind = FABRIC_SECURITY_CREDENTIAL_KIND_NONE;
            let mut settings = FABRIC_TRANSPORT_SETTINGS::default();
            settings.KeepAliveTimeoutInSeconds = 10;
            settings.MaxConcurrentCalls = 10;
            settings.MaxMessageSize = 10;
            settings.MaxQueueSize = 10;
            settings.OperationTimeoutInSeconds = 10;
            settings.SecurityCredentials = &creds;

            // create server
            let mut serveraddr = FABRIC_TRANSPORT_LISTEN_ADDRESS::default();
            let host = HSTRING::from("localhost");
            let path = HSTRING::from("/");
            serveraddr.IPAddressOrFQDN = PCWSTR::from(&host);
            serveraddr.Port = 12345;
            serveraddr.Path = PCWSTR::from(&path);
            listener = ServerTransport::new(&settings, &serveraddr).unwrap();
        }

        let listen_addr = listener.open().await.unwrap();

        let connectionaddress = HSTRING::from("localhost:12345+/");
        assert_eq!(listen_addr, connectionaddress);

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
            });
        }
        //tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        listener.close().await.unwrap();
    });

    let timoutmilliseconds = 100000;
    let connectionaddress = HSTRING::from("localhost:12345+/");
    let mut client = ClientTransport::new(&settings, &connectionaddress).unwrap();
    client.open(timoutmilliseconds).await.unwrap();

    // This wait is optional in prod
    client.connect().await;

    // send request
    {
        let header = String::from("myheader");
        let body = String::from("mybody");
        let msg = Message::create(header.clone().into_bytes(), body.clone().into_bytes());
        let reply = client.request(timoutmilliseconds, &msg).await.unwrap();
        let replyvw = MessageViewer::new(reply);

        let header_ret = replyvw.get_header();
        assert_eq!(header_ret, String::from("hello: myheader").as_bytes());

        let body_ret = replyvw.get_body();
        assert_eq!(body_ret, String::from("hello: mybody").as_bytes());
    }

    // stop server
    stoptx.send(()).unwrap();

    // this wait is optional in prod
    let hr = client.disconnect().await;
    assert_eq!(hr, HRESULT(FABRIC_E_CONNECTION_CLOSED_BY_REMOTE_END.0));

    // close client
    client.close(timoutmilliseconds).await.unwrap();
}
