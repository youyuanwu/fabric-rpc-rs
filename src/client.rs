// client for fabric-rpc protocol

use std::io::Cursor;

use prost::Message;
use tonic::{Code, Status};

use crate::{
    client_tr::ClientTransport,
    fabricrpc_header::{ReplyHeader, RequestHeader},
    sys::MessageViewer,
};

// Client is a wrapper for the transport to implement rpc protocol
pub struct Client<'a> {
    tr: &'a ClientTransport,
}

impl Client<'_> {
    pub fn new(tr: &ClientTransport) -> Client<'_> {
        Client { tr }
    }

    // send the msg and returns the proto reply
    pub async fn request<T: Message + std::default::Default>(
        &self,
        url: String,
        msg: &impl Message,
        timoutmilliseconds: u32,
    ) -> Result<T, Status> {
        let reqheader = RequestHeader { url };

        let mut headerbuf = Vec::new();
        reqheader.encode(&mut headerbuf).unwrap();

        let mut bodybuf = Vec::new();
        msg.encode(&mut bodybuf).unwrap();

        let msg = crate::sys::Message::create(headerbuf, bodybuf);
        let reply = self.tr.request(timoutmilliseconds, &msg).await;
        if reply.is_err() {
            return Err(Status::internal(reply.unwrap_err().message().to_string()));
        }

        let reply = reply.unwrap();

        let replyvw = MessageViewer::new(reply);
        let header_ret = replyvw.get_header();
        let body_ret = replyvw.get_body();

        let replyheader = ReplyHeader::decode(&mut Cursor::new(header_ret));

        if let Err(err) = replyheader {
            return Err(Status::internal(err.to_string()));
        }

        let replyheader = replyheader.unwrap();
        let code = replyheader.status_code;
        let status_msg = replyheader.status_message;
        let code_enum = Code::from_i32(code);
        let headerstatus = Status::new(code_enum, status_msg);
        if headerstatus.code() != Code::Ok {
            return Err(headerstatus);
        }

        let replyout = T::decode(&mut Cursor::new(body_ret));

        if let Err(err) = replyout {
            return Err(Status::internal(err.to_string()));
        }
        Ok(replyout.unwrap())
    }
}
