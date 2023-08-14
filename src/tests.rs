use service_fabric_rs::{
    FabricCommon::FabricTransport::{FABRIC_TRANSPORT_LISTEN_ADDRESS, FABRIC_TRANSPORT_SETTINGS},
    FABRIC_SECURITY_CREDENTIALS, FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
};
use windows::core::{HSTRING, PCWSTR};

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

    let listener = ServerTransport::new(&settings, &serveraddr).unwrap();
    let listen_addr = listener.open().await.unwrap();

    let connectionaddress = HSTRING::from("localhost:12345+/");
    assert_eq!(listen_addr, connectionaddress);

    let client = ClientTransport::new(&settings, &connectionaddress).unwrap();
    client.open().await.unwrap();

    // send request
    {
        let header = String::from("myheader");
        let body = String::from("mybody");
        let msg = Message::create(header.clone().into_bytes(), body.clone().into_bytes());
        let reply = client.request(&msg).await.unwrap();
        let replyvw = MessageViewer::new(reply);

        let header_ret = replyvw.get_header();
        assert_eq!(header_ret, String::from("hello: myheader").as_bytes());

        let body_ret = replyvw.get_body();
        assert_eq!(body_ret, String::from("hello: mybody").as_bytes());
    }

    // close client
    client.close().await.unwrap();

    // close server
    listener.close().await.unwrap();
}
