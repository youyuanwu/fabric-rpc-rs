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

#[cfg(test)]
mod hello_test {

    use windows::core::{Error, HSTRING};

    use crate::{
        client::Client2,
        server::{encode_proto, parse_proto, Server, Service},
    };

    use super::test_grpc::hello_world::{HelloReply, HelloRequest};

    // User needs to implement
    #[tonic::async_trait]
    trait HelloService: Send + Sync + 'static {
        async fn say_hello(request: HelloRequest) -> Result<HelloReply, tonic::Status>;
    }

    struct HelloSvcImpl {}

    #[tonic::async_trait]
    impl HelloService for HelloSvcImpl {
        async fn say_hello(request: HelloRequest) -> Result<HelloReply, tonic::Status> {
            let name = request.name;
            let mut msg_reply = String::from("Hello: ");
            msg_reply.push_str(name.as_str());
            let reply = HelloReply { message: msg_reply };
            Ok(reply)
        }
    }

    // this is auto generated
    // HelloServiceRouter is used for routing
    struct HelloServiceRouter<T: HelloService> {
        _svc: T, // ???why not used
    }

    impl<T: HelloService> HelloServiceRouter<T> {
        fn new(svc: T) -> HelloServiceRouter<T> {
            HelloServiceRouter { _svc: svc }
        }
    }

    #[tonic::async_trait]
    impl<T: HelloService> Service for HelloServiceRouter<T> {
        fn name(&self) -> String {
            return String::from("helloworld.Greeter");
        }

        #[must_use]
        async fn handle_request(
            &self,
            url: String,
            request: &[u8],
        ) -> std::result::Result<Vec<u8>, tonic::Status> {
            match url.as_str() {
                "/helloworld.Greeter/SayHello" => {
                    let req = parse_proto(request)?;
                    let resp = T::say_hello(req).await?;
                    return encode_proto(&resp);
                }
                _ => Err(tonic::Status::unimplemented("url not found")),
            }
        }
    }

    // hello client
    struct HelloClient {
        c: Client2,
    }

    impl HelloClient {
        pub async fn connect(addr: HSTRING) -> Result<HelloClient, Error> {
            let c = Client2::connect(addr).await?;
            Ok(HelloClient { c })
        }

        pub async fn say_hello(
            &self,
            timoutmilliseconds: u32,
            request: HelloRequest,
        ) -> Result<HelloReply, tonic::Status> {
            let url = String::from("/helloworld.Greeter/SayHello");
            return self.c.request(url, &request, timoutmilliseconds).await;
        }
    }

    #[tokio::test]
    async fn test_fabricrpc_helloworld() {
        let (stoptx, stoprx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            // make server run
            let hello_svc = HelloSvcImpl {};

            let mut svr = Server::default();
            svr.add_service(HelloServiceRouter::new(hello_svc));
            svr.serve_with_shutdown(12346, stoprx).await;
        });

        let connectionaddress = HSTRING::from("localhost:12346+/");

        let helloclient = HelloClient::connect(connectionaddress).await.unwrap();

        // // send request
        let request = HelloRequest {
            name: String::from("myname"),
        };
        let resp = helloclient.say_hello(1000, request).await.unwrap();

        assert_eq!("Hello: myname", resp.message);

        // stop server
        stoptx.send(()).unwrap();
    }
}

#[cfg(test)]
mod test_grpc {

    use tokio::sync::oneshot;
    use tonic::{transport::Server, Request, Response, Status};

    use hello_world::greeter_client::GreeterClient;
    use hello_world::greeter_server::{Greeter, GreeterServer};
    use hello_world::{HelloReply, HelloRequest};

    #[allow(non_snake_case)]
    pub mod hello_world {
        tonic::include_proto!("helloworld"); // The string specified here must match the proto package name
    }

    #[derive(Debug, Default)]
    pub struct MyGreeter {}

    #[tonic::async_trait]
    impl Greeter for MyGreeter {
        async fn say_hello(
            &self,
            request: Request<HelloRequest>, // Accept request of type HelloRequest
        ) -> Result<Response<HelloReply>, Status> {
            // Return an instance of type HelloReply
            println!("Got a request: {:?}", request);

            let reply = hello_world::HelloReply {
                message: format!("Hello {}!", request.into_inner().name).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
            };

            Ok(Response::new(reply)) // Send back our formatted greeting
        }
    }

    #[tokio::test]
    async fn test_server() {
        let (tx, rx) = oneshot::channel::<()>();

        tokio::spawn(async {
            let addr = "[::1]:50051".parse().unwrap();
            let greeter = MyGreeter::default();

            Server::builder()
                .add_service(GreeterServer::new(greeter))
                .serve_with_shutdown(addr, async {
                    rx.await.ok();
                    println!("Graceful shutdown complete")
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut client = GreeterClient::connect("http://[::1]:50051").await.unwrap();

        let request = tonic::Request::new(HelloRequest {
            name: "Tonic".into(),
        });

        let response = client.say_hello(request).await.unwrap();

        println!("RESPONSE={:?}", response);
        tx.send(()).unwrap();
    }
}

#[cfg(test)]
mod generator_test {
    #[allow(non_snake_case)]
    pub mod fabrichello {
        tonic::include_proto!("fabrichello"); // The string specified here must match the proto package name
    }

    #[test]
    fn mytest() {
        let _ = fabrichello::MyClientTest {};
    }
}
