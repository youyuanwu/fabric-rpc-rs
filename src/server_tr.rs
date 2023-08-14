use service_fabric_rs::FabricCommon::{
    FabricTransport::{
        CreateFabricTransportListener, IFabricTransportClientConnection,
        IFabricTransportConnectionHandler, IFabricTransportConnectionHandler_Impl,
        IFabricTransportListener, IFabricTransportMessage, IFabricTransportMessageDisposer,
        IFabricTransportMessageHandler, IFabricTransportMessageHandler_Impl,
        FABRIC_TRANSPORT_LISTEN_ADDRESS, FABRIC_TRANSPORT_SETTINGS,
    },
    IFabricAsyncOperationCallback, IFabricAsyncOperationContext, IFabricAsyncOperationContext_Impl,
};
use windows::{
    core::{implement, Error, Interface, HSTRING},
    Win32::Foundation::E_POINTER,
};

use crate::{
    shared_tr::MsgDispoer,
    sys::{AwaitableCallback, Context, Message, MessageViewer, StringViewer},
};

// server code
#[derive(Debug)]
#[implement(IFabricTransportConnectionHandler)]
pub struct ServerConnHandler {}

impl ServerConnHandler {
    pub fn new() -> ServerConnHandler {
        ServerConnHandler {}
    }
}

#[allow(non_snake_case)]
impl IFabricTransportConnectionHandler_Impl for ServerConnHandler {
    fn BeginProcessConnect(
        &self,
        _clientconnection: &::core::option::Option<IFabricTransportClientConnection>,
        _timeoutmilliseconds: u32,
        callback: &::core::option::Option<IFabricAsyncOperationCallback>,
    ) -> ::windows::core::Result<IFabricAsyncOperationContext> {
        if let Some(cb) = callback {
            let mut ctx = Context::new(cb.clone());
            ctx.complete();
            unsafe { cb.Invoke(&ctx.clone().into()) };
            Ok(ctx.into())
        } else {
            Err(E_POINTER.into())
        }
    }

    fn EndProcessConnect(
        &self,
        context: &::core::option::Option<IFabricAsyncOperationContext>,
    ) -> ::windows::core::Result<()> {
        if let Some(ctx) = context {
            let cast = Context::from_interface(ctx);
            assert!(cast.IsCompleted().as_bool());
            Ok(())
        } else {
            Err(E_POINTER.into())
        }
    }

    fn BeginProcessDisconnect(
        &self,
        _clientid: *const u16,
        _timeoutmilliseconds: u32,
        callback: &::core::option::Option<IFabricAsyncOperationCallback>,
    ) -> ::windows::core::Result<IFabricAsyncOperationContext> {
        if let Some(cb) = callback {
            let mut ctx = Context::new(cb.clone());
            ctx.complete();
            unsafe { cb.Invoke(&ctx.clone().into()) };
            Ok(ctx.into())
        } else {
            Err(E_POINTER.into())
        }
    }

    fn EndProcessDisconnect(
        &self,
        context: &::core::option::Option<IFabricAsyncOperationContext>,
    ) -> ::windows::core::Result<()> {
        if let Some(ctx) = context {
            let cast = Context::from_interface(ctx);
            assert!(cast.IsCompleted().as_bool());
            Ok(())
        } else {
            Err(E_POINTER.into())
        }
    }
}

#[derive(Debug)]
#[implement(IFabricTransportMessageHandler)]
pub struct MessageHandler {}

impl MessageHandler {
    pub fn new() -> MessageHandler {
        MessageHandler {}
    }
}

#[allow(non_snake_case)]
impl IFabricTransportMessageHandler_Impl for MessageHandler {
    fn BeginProcessRequest(
        &self,
        _clientid: *const u16,
        message: &::core::option::Option<IFabricTransportMessage>,
        _timeoutmilliseconds: u32,
        callback: &::core::option::Option<IFabricAsyncOperationCallback>,
    ) -> ::windows::core::Result<IFabricAsyncOperationContext> {
        if message.is_none() || callback.is_none() {
            return Err(E_POINTER.into());
        }

        let msg: &IFabricTransportMessage = message.as_ref().unwrap();
        let cb = callback.as_ref().unwrap();

        let mut ctx = Context::new(cb.clone());

        let vw = MessageViewer::new(msg.clone());

        let header = vw.get_header();
        let body = vw.get_body();

        let hello = String::from("hello: ").into_bytes();
        let mut reply_header = hello.clone();
        reply_header.extend(header);
        let mut reply_body = hello.clone();
        reply_body.extend(body);

        let reply = Message::create(reply_header, reply_body);

        ctx.set_msg(reply);
        ctx.complete();
        unsafe { cb.Invoke(&ctx.clone().into()) };

        Ok(ctx.into())
    }

    fn EndProcessRequest(
        &self,
        context: &::core::option::Option<IFabricAsyncOperationContext>,
    ) -> ::windows::core::Result<IFabricTransportMessage> {
        if context.is_none() {
            return Err(E_POINTER.into());
        }

        let ctx = context.as_ref().unwrap().clone();
        let cast = Context::from_interface(&ctx);
        let msg = cast.get_msg().unwrap();
        Ok(msg)
    }

    fn HandleOneWay(
        &self,
        _clientid: *const u16,
        _message: &::core::option::Option<IFabricTransportMessage>,
    ) -> ::windows::core::Result<()> {
        Ok(())
    }
}

// server
pub struct ServerTransport {
    l: IFabricTransportListener,
}

impl ServerTransport {
    pub fn new(
        settings: &FABRIC_TRANSPORT_SETTINGS,
        address: &FABRIC_TRANSPORT_LISTEN_ADDRESS,
    ) -> Result<ServerTransport, Error> {
        let disposeprocessor: IFabricTransportMessageDisposer = MsgDispoer::new().into();
        let svr_conn_handler: IFabricTransportConnectionHandler = ServerConnHandler::new().into();
        let msg_handler: IFabricTransportMessageHandler = MessageHandler::new().into();
        let listener = unsafe {
            CreateFabricTransportListener(
                &IFabricTransportListener::IID,
                settings,
                address,
                &msg_handler,
                &svr_conn_handler,
                &disposeprocessor,
            )?
        };
        Ok(ServerTransport { l: listener })
    }

    pub async fn open(&self) -> Result<HSTRING, Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.l.BeginOpen(&callback) }?;
        rx.await.unwrap();
        let addr = unsafe { self.l.EndOpen(&ctx) }?;
        let sv = StringViewer::new(addr);
        Ok(sv.get_hstring())
    }

    pub async fn close(&self) -> Result<(), Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.l.BeginClose(&callback) }?;
        rx.await.unwrap();
        unsafe { self.l.EndClose(&ctx) }?;

        Ok(())
    }
}
