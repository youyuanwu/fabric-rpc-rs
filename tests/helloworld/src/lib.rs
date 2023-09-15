use gen::fabric_hello_server;

#[allow(non_snake_case)]
pub mod gen {
    tonic::include_proto!("fabrichello"); // The string specified here must match the proto package name
}

pub struct HelloSvcImpl {}

#[tonic::async_trait]
impl fabric_hello_server::FabricHelloService for HelloSvcImpl {
    async fn say_hello(request: gen::FabricRequest) -> Result<gen::FabricResponse, tonic::Status> {
        let name = request.fabric_name;
        let mut msg_reply = String::from("Hello: ");
        msg_reply.push_str(name.as_str());
        let reply = gen::FabricResponse {
            fabric_message: msg_reply,
        };
        Ok(reply)
    }
}

#[cfg(test)]
mod generator_test {
    use fabric_rpc_rs::server::Server;
    use windows::core::HSTRING;

    use crate::{
        gen::{fabric_hello_client::FabricHelloClient, fabric_hello_server, FabricRequest},
        HelloSvcImpl,
    };

    #[tokio::test]
    async fn mytest() {
        // open server
        let (stoptx, stoprx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            // make server run
            let hello_svc = HelloSvcImpl {};

            let mut svr = Server::default();
            svr.add_service(fabric_hello_server::FabricHelloServiceRouter::new(
                hello_svc,
            ));
            svr.serve_with_shutdown(12347, stoprx).await;
        });

        let connectionaddress = HSTRING::from("localhost:12347+/");

        let helloclient = FabricHelloClient::connect(connectionaddress).await.unwrap();

        // send request
        let request = FabricRequest {
            fabric_name: String::from("myname"),
        };
        let resp = helloclient.say_hello(1000, request).await.unwrap();

        assert_eq!("Hello: myname", resp.fabric_message);

        // stop server
        stoptx.send(()).unwrap();
    }
}
