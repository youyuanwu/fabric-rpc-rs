// server

use std::{future::Future, sync::Arc};

use prost::Message;
use service_fabric_rs::{
    FabricCommon::FabricTransport::{FABRIC_TRANSPORT_LISTEN_ADDRESS, FABRIC_TRANSPORT_SETTINGS},
    FABRIC_SECURITY_CREDENTIALS, FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
};
use tonic::async_trait;
use windows::core::{HSTRING, PCWSTR};

use crate::{
    fabricrpc_header::{ReplyHeader, RequestHeader},
    server_tr::ServerTransport,
    sys::MessageViewer,
};

#[derive(Default)]
pub struct Server {
    svcs: Vec<Box<dyn Service>>,
}

struct ServerInner {
    svcs: Arc<Vec<Box<dyn Service>>>,
}

impl Server {
    // pub fn build(self) -> ServerInner{
    //   ServerInner { svcs: Arc::new(self.svcs) }
    // }

    pub fn add_service<T: Service + 'static>(&mut self, svc: T) {
        self.svcs.push(Box::new(svc));
    }

    pub async fn serve_with_shutdown<F: Future<Output = ()>>(self, port: u32, signal: F) {
        let mut inner = ServerInner {
            svcs: Arc::new(self.svcs),
        };
        inner.serve_with_shutdown(port, signal).await
    }
}

impl ServerInner {
    // internal execute request
    async fn execute(&mut self, msgvw: MessageViewer<'_>) -> Result<Vec<u8>, tonic::Status> {
        //let msgvw = MessageViewer::new(msg);
        let header_buff = msgvw.get_header();
        let body_buff = msgvw.get_body();

        let header = RequestHeader::decode(header_buff);
        if let Err(err) = header {
            let mut err_str = String::from("header invalid, failed to parse");
            err_str.push_str(&err.to_string());
            return Err(tonic::Status::invalid_argument(err_str));
        }

        let url = header.unwrap().url;
        if url.is_empty() || !url.starts_with('/') {
            return Err(tonic::Status::invalid_argument("url not valid"));
        }

        let url_without_prefix = &url.as_bytes()[1..];

        for svc in self.svcs.iter() {
            let svc_url = svc.name();
            if !url_without_prefix.starts_with(svc_url.as_bytes()) {
                continue;
            }
            let result = svc.handle_request(url, body_buff).await?;
            return Ok(result);
        }
        Err(tonic::Status::unimplemented("url not found"))
    }

    async fn serve_with_shutdown<F>(&mut self, port: u32, signal: F)
    where
        F: Future<Output = ()>,
    {
        let mut listener: ServerTransport;
        {
            let creds = FABRIC_SECURITY_CREDENTIALS {
                Kind: FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
                Value: std::ptr::null_mut(),
            };
            let settings = FABRIC_TRANSPORT_SETTINGS {
                OperationTimeoutInSeconds: 10,
                KeepAliveTimeoutInSeconds: 10,
                MaxMessageSize: 1024,
                MaxConcurrentCalls: 10,
                MaxQueueSize: 10,
                SecurityCredentials: &creds,
                Reserved: std::ptr::null_mut(),
            };

            // create server
            let mut serveraddr = FABRIC_TRANSPORT_LISTEN_ADDRESS::default();
            let host = HSTRING::from("localhost");
            let path = HSTRING::from("/");
            serveraddr.IPAddressOrFQDN = PCWSTR::from(&host);
            serveraddr.Port = port;
            serveraddr.Path = PCWSTR::from(&path);
            listener = ServerTransport::new(&settings, &serveraddr).unwrap();
        }

        let _ = listener.open().await.unwrap();

        //let connectionaddress = HSTRING::from("localhost:12345+/");
        // assert_eq!(listen_addr, connectionaddress);

        let mut p = Box::pin(signal);
        loop {
            let mut conn;
            tokio::select! {
                _ = (&mut p) => { break;},
                x = listener.async_accept() => {
                    conn = x;
                }
            }
            //println!("Server got connection");

            let mut inner_clone = ServerInner {
                svcs: self.svcs.clone(),
            };

            tokio::spawn(async move {
                // loop until the request from this server is drained.
                loop {
                    let req = conn.async_accept().await;
                    if req.is_none() {
                        break;
                    }
                    let mut req = req.unwrap();
                    //println!("Server got request");

                    let vw;
                    {
                        let msg = req.get_request_msg();
                        vw = MessageViewer::new(msg.clone());
                    }
                    // let header = vw.get_header();
                    // let body = vw.get_body();

                    let payload = inner_clone.execute(vw).await;

                    let mut replyheader = ReplyHeader::default();
                    let mut replybody = Vec::new();
                    match payload {
                        Err(st) => {
                            replyheader.status_code = st.code() as i32;
                            replyheader.status_message = String::from(st.message());
                        }
                        Ok(content) => {
                            replyheader.status_code = tonic::Code::Ok as i32;
                            replyheader.status_message = String::from("Ok");
                            replybody = content;
                        }
                    }

                    let header_buff = encode_proto(&replyheader).unwrap();

                    let reply = crate::sys::Message::create(header_buff, replybody);
                    req.complete(reply);
                }
            });
        }
        //tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        //println!("Server shutdown");
        listener.close().await.unwrap();
    }
}

// parse proto from bytes
pub fn parse_proto<T: prost::Message + Default>(buf: &[u8]) -> Result<T, tonic::Status> {
    let proto = T::decode(buf);
    if let Err(err) = proto {
        let mut err_str = String::from("failed to parse proto: "); // TODO add proto name?
        err_str.push_str(&err.to_string());
        return Err(tonic::Status::invalid_argument(err_str));
    }
    Ok(proto.unwrap())
}

pub fn encode_proto<T: prost::Message>(proto: &T) -> Result<Vec<u8>, tonic::Status> {
    let mut buf = Vec::new();
    let err = proto.encode(&mut buf);
    if let Err(e) = err {
        let mut err_str = String::from("failed to encode proto: ");
        err_str.push_str(&e.to_string());
        return Err(tonic::Status::internal(err_str));
    }
    Ok(buf)
}

// Each rpc service needs to implement this
#[async_trait]
pub trait Service: Send + Sync {
    fn name(&self) -> String;
    async fn handle_request(
        &self,
        url: String,
        request: &[u8],
    ) -> std::result::Result<Vec<u8>, tonic::Status>;
}
