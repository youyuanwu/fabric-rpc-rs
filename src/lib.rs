use std::cell::RefCell;

use service_fabric_rs::FabricCommon::{
    FabricTransport::{
        IFabricTransportMessage, IFabricTransportMessage_Impl, FABRIC_TRANSPORT_MESSAGE_BUFFER,
    },
    IFabricAsyncOperationCallback, IFabricAsyncOperationCallback_Impl,
    IFabricAsyncOperationContext, IFabricAsyncOperationContext_Impl,
};

use tokio::sync::oneshot::{self, Receiver, Sender};
use windows::core::{implement, AsImpl};

#[allow(
    //non_camel_case_types,
    non_snake_case,
)]
// awaitable callback is used to await a signal from fabric API.
#[implement(IFabricAsyncOperationCallback)]
pub struct AwaitableCallback {
    tx: RefCell<Option<Sender<()>>>, // invoke has self immutable so use use RefCell
}

impl AwaitableCallback {
    pub fn create() -> (IFabricAsyncOperationCallback, Receiver<()>) {
        let (tx, rx) = oneshot::channel::<()>();

        let callback = AwaitableCallback {
            tx: RefCell::new(Some(tx)),
        };

        let cast: IFabricAsyncOperationCallback = callback.into();
        (cast, rx)
    }
}

#[allow(non_snake_case)]
impl IFabricAsyncOperationCallback_Impl for AwaitableCallback {
    fn Invoke(&self, _context: &::core::option::Option<IFabricAsyncOperationContext>) {
        let op = self.tx.take();
        if let Some(send) = op {
            send.send(()).unwrap();
        }
    }
}

#[derive(Clone)]
#[implement(IFabricAsyncOperationContext)]
pub struct Context {
    completed: bool,
    callback: IFabricAsyncOperationCallback,
    msg: Option<IFabricTransportMessage>,
}

impl Context {
    pub fn new(callback: IFabricAsyncOperationCallback) -> Context {
        Context {
            completed: false,
            callback,
            msg: None,
        }
    }

    // get a view of Context from interface.
    // This is unsafe. User needs to ensure that arg is of type context
    pub fn from_interface(ctx: &IFabricAsyncOperationContext) -> &Context {
        let inner = ctx.as_impl();
        inner
    }

    pub fn complete(&mut self) {
        assert!(!self.completed);
        self.completed = true;
    }

    // pub fn complete(&mut self) {
    //     let copy = self.into_interface().clone();
    //     self.invoke(&copy);
    //     self.completed = true;
    // }

    // this is like query interface
    // pub fn into_interface(&self) -> IFabricAsyncOperationContext {
    //     unsafe{self.cast()}.unwrap()
    // }

    // returns a cast type. TODO: maybe not safe
    // the returned type does not have ownership?
    // fn as_interface(&self) -> IFabricAsyncOperationContext {
    //     unsafe { self.cast::<IFabricAsyncOperationContext>() }.unwrap()
    // }

    pub fn set_msg(&mut self, msg: IFabricTransportMessage) {
        assert_eq!(self.msg, None);
        assert!(!self.completed);
        self.msg = Some(msg);
    }

    // once msg is get the context should have no use
    pub fn get_msg(&self) -> Option<IFabricTransportMessage> {
        assert!(self.completed);
        self.msg.clone()
    }
}

#[allow(non_snake_case)]
impl IFabricAsyncOperationContext_Impl for Context {
    fn IsCompleted(&self) -> ::windows::Win32::Foundation::BOOLEAN {
        self.completed.into()
    }

    // always return false.
    fn CompletedSynchronously(&self) -> ::windows::Win32::Foundation::BOOLEAN {
        false.into()
    }

    fn Callback(&self) -> ::windows::core::Result<IFabricAsyncOperationCallback> {
        Ok(self.callback.clone())
    }

    fn Cancel(&self) -> ::windows::core::Result<()> {
        // does not support cancel
        Ok(())
    }
}

#[implement(IFabricTransportMessage)]
pub struct Message {
    header: Vec<u8>,
    body: Vec<u8>,
    header_buff: FABRIC_TRANSPORT_MESSAGE_BUFFER,
    body_buff: FABRIC_TRANSPORT_MESSAGE_BUFFER,
}

impl Message {
    pub fn create(header: Vec<u8>, body: Vec<u8>) -> IFabricTransportMessage {
        let mut msg = Message {
            header,
            body,
            header_buff: Default::default(),
            body_buff: Default::default(),
        };

        let h = &msg.header;
        msg.header_buff.BufferSize = h.len() as u32;
        msg.header_buff.Buffer = h.as_ptr() as *mut u8;

        let b = &msg.body;
        msg.body_buff.BufferSize = b.len() as u32;
        msg.body_buff.Buffer = b.as_ptr() as *mut u8;

        msg.into()
    }
}

#[allow(non_snake_case)]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // public def does not have unsafe
impl IFabricTransportMessage_Impl for Message {
    fn GetHeaderAndBodyBuffer(
        &self,
        headerbuffer: *mut *mut FABRIC_TRANSPORT_MESSAGE_BUFFER,
        msgbuffercount: *mut u32,
        msgbuffers: *mut *mut FABRIC_TRANSPORT_MESSAGE_BUFFER,
    ) {
        if headerbuffer.is_null() || msgbuffercount.is_null() || msgbuffers.is_null() {
            return;
        }
        unsafe {
            *headerbuffer =
                std::ptr::addr_of!(self.header_buff) as *mut FABRIC_TRANSPORT_MESSAGE_BUFFER;
            *msgbuffercount = 1;
            *msgbuffers =
                std::ptr::addr_of!(self.body_buff) as *mut FABRIC_TRANSPORT_MESSAGE_BUFFER;
        }
    }

    fn Dispose(&self) {}
}

// viewing the msg
pub struct MessageViewer<'a> {
    _msg: IFabricTransportMessage,
    header: &'a [u8],
    body: &'a [u8],
}

impl MessageViewer<'_> {
    pub fn new(msg: IFabricTransportMessage) -> MessageViewer<'static> {
        //MessageViewer{msg}
        let mut header_buff: *mut FABRIC_TRANSPORT_MESSAGE_BUFFER = std::ptr::null_mut();
        let mut body_buff: *mut FABRIC_TRANSPORT_MESSAGE_BUFFER = std::ptr::null_mut();
        let mut body_count: u32 = 0;

        unsafe {
            msg.GetHeaderAndBodyBuffer(
                std::ptr::addr_of_mut!(header_buff),
                std::ptr::addr_of_mut!(body_count),
                std::ptr::addr_of_mut!(body_buff),
            )
        };
        let mut header_slice: &[u8] = &[];
        let mut body_slice: &[u8] = &[];
        if !header_buff.is_null() {
            let header_ref = unsafe { &*header_buff };

            header_slice = unsafe {
                std::slice::from_raw_parts(header_ref.Buffer, header_ref.BufferSize as usize)
            };
        }
        if body_count != 0 && !body_buff.is_null() {
            let body_ref = unsafe { &*body_buff };

            body_slice = unsafe {
                std::slice::from_raw_parts(body_ref.Buffer, body_ref.BufferSize as usize)
            };
        }

        MessageViewer {
            _msg: msg,
            header: header_slice,
            body: body_slice,
        }
    }

    pub fn get_header(&self) -> &'_ [u8] {
        self.header
    }

    pub fn get_body(&self) -> &'_ [u8] {
        self.body
    }
}

#[cfg(test)]
mod tests {

    #[allow(
        //non_camel_case_types,
        non_snake_case,
    )]
    use service_fabric_rs::FabricCommon::FabricTransport::{
        CreateFabricTransportClient, IFabricTransportCallbackMessageHandler,
        IFabricTransportCallbackMessageHandler_Impl, IFabricTransportClientEventHandler,
        IFabricTransportMessage, IFabricTransportMessageDisposer, IFabricTransportMessageHandler,
        FABRIC_TRANSPORT_SETTINGS,
    };
    use service_fabric_rs::{
        FabricCommon::FabricTransport::{
            CreateFabricTransportListener, IFabricTransportClient,
            IFabricTransportClientConnection, IFabricTransportClientEventHandler_Impl,
            IFabricTransportConnectionHandler, IFabricTransportConnectionHandler_Impl,
            IFabricTransportListener, IFabricTransportMessageDisposer_Impl,
            IFabricTransportMessageHandler_Impl, FABRIC_TRANSPORT_LISTEN_ADDRESS,
        },
        FABRIC_SECURITY_CREDENTIALS, FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
    };
    use windows::{
        core::{implement, Interface, HSTRING, PCWSTR},
        Win32::Foundation::E_POINTER,
    };

    use super::*;

    #[tokio::test]
    async fn test_callback() {
        let (callback, rx) = AwaitableCallback::create();
        unsafe {
            callback.Invoke(None);
        }
        rx.await.unwrap();
    }

    #[tokio::test]
    async fn test_ctx() {
        let (callback, rx) = AwaitableCallback::create();
        let mut ctx = Context::new(callback.clone());
        unsafe { callback.Invoke(&ctx.clone().into()) };
        ctx.complete();
        rx.await.unwrap();
        let ictx: IFabricAsyncOperationContext = ctx.into();

        let cast = Context::from_interface(&ictx);
        assert!(cast.completed);
    }

    #[test]
    fn test_msg() {
        let header = String::from("myheader");
        let body = String::from("mybody");
        let msg = Message::create(header.clone().into_bytes(), body.clone().into_bytes());

        let msgvw = MessageViewer::new(msg);
        let header_ret = msgvw.get_header();
        assert_eq!(header_ret, header.as_bytes());

        let body_ret = msgvw.get_body();
        assert_eq!(body_ret, body.as_bytes());
    }

    #[derive(Debug)]
    #[implement(IFabricTransportCallbackMessageHandler)]
    pub struct ClientMsgHandler {}

    impl ClientMsgHandler {
        pub fn new() -> ClientMsgHandler {
            ClientMsgHandler {}
        }
    }

    #[allow(non_snake_case)]
    impl IFabricTransportCallbackMessageHandler_Impl for ClientMsgHandler {
        fn HandleOneWay(
            &self,
            _message: &::core::option::Option<IFabricTransportMessage>,
        ) -> ::windows::core::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug)]
    #[implement(IFabricTransportClientEventHandler)]
    pub struct ClientEvHandler {}

    impl ClientEvHandler {
        pub fn new() -> ClientEvHandler {
            ClientEvHandler {}
        }
    }

    #[allow(non_snake_case)]
    impl IFabricTransportClientEventHandler_Impl for ClientEvHandler {
        fn OnConnected(
            &self,
            _connectionaddress: &::windows::core::PCWSTR,
        ) -> ::windows::core::Result<()> {
            Ok(())
        }

        fn OnDisconnected(
            &self,
            _connectionaddress: &::windows::core::PCWSTR,
            _error: ::windows::core::HRESULT,
        ) -> ::windows::core::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug)]
    #[implement(IFabricTransportMessageDisposer)]
    pub struct MsgDispoer {}

    impl MsgDispoer {
        pub fn new() -> MsgDispoer {
            MsgDispoer {}
        }
    }

    #[allow(non_snake_case)]
    impl IFabricTransportMessageDisposer_Impl for MsgDispoer {
        fn Dispose(
            &self,
            _count: u32,
            _messages: *const ::core::option::Option<IFabricTransportMessage>,
        ) {
            ()
        }
    }

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
            callback: &::core::option::Option<super::IFabricAsyncOperationCallback>,
        ) -> ::windows::core::Result<super::IFabricAsyncOperationContext> {
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
            context: &::core::option::Option<super::IFabricAsyncOperationContext>,
        ) -> ::windows::core::Result<()> {
            if let Some(ctx) = context {
                let cast = Context::from_interface(ctx);
                assert!(cast.completed);
                Ok(())
            } else {
                Err(E_POINTER.into())
            }
        }

        fn BeginProcessDisconnect(
            &self,
            _clientid: *const u16,
            _timeoutmilliseconds: u32,
            callback: &::core::option::Option<super::IFabricAsyncOperationCallback>,
        ) -> ::windows::core::Result<super::IFabricAsyncOperationContext> {
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
            context: &::core::option::Option<super::IFabricAsyncOperationContext>,
        ) -> ::windows::core::Result<()> {
            if let Some(ctx) = context {
                let cast = Context::from_interface(ctx);
                assert!(cast.completed);
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
            callback: &::core::option::Option<super::IFabricAsyncOperationCallback>,
        ) -> ::windows::core::Result<super::IFabricAsyncOperationContext> {
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
            context: &::core::option::Option<super::IFabricAsyncOperationContext>,
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

        let disposeprocessor: IFabricTransportMessageDisposer = MsgDispoer::new().into();
        let svr_conn_handler: IFabricTransportConnectionHandler = ServerConnHandler::new().into();
        let msg_handler: IFabricTransportMessageHandler = MessageHandler::new().into();
        let listener: IFabricTransportListener = unsafe {
            CreateFabricTransportListener(
                &IFabricTransportListener::IID,
                &settings,
                &serveraddr,
                &msg_handler,
                &svr_conn_handler,
                &disposeprocessor,
            )
            .unwrap()
        };

        // open server
        {
            let (callback, rx) = AwaitableCallback::create();
            let ctx = unsafe { listener.BeginOpen(&callback) }.unwrap();
            rx.await.unwrap();
            let _addr = unsafe { listener.EndOpen(&ctx) }.unwrap();
        }

        let connectionaddress = HSTRING::from("localhost:12345+/");
        let notificationhandler: IFabricTransportCallbackMessageHandler =
            ClientMsgHandler::new().into();
        let clienteventhandler: IFabricTransportClientEventHandler = ClientEvHandler::new().into();
        let messagedisposer: IFabricTransportMessageDisposer = MsgDispoer::new().into();

        // create client
        let client = unsafe {
            CreateFabricTransportClient(
                &IFabricTransportClient::IID,
                &settings,
                &connectionaddress,
                &notificationhandler,
                &clienteventhandler,
                &messagedisposer,
            )
        }
        .unwrap();

        // open client
        {
            let (callback, rx) = AwaitableCallback::create();
            let ctx = unsafe { client.BeginOpen(1000, &callback) }.unwrap();
            rx.await.unwrap();
            unsafe { client.EndOpen(&ctx) }.unwrap();
        }

        // send request
        {
            let header = String::from("myheader");
            let body = String::from("mybody");
            let (callback, rx) = AwaitableCallback::create();
            let msg = Message::create(header.clone().into_bytes(), body.clone().into_bytes());
            let ctx = unsafe { client.BeginRequest(&msg, 1000, &callback) }.unwrap();
            rx.await.unwrap();
            let reply = unsafe { client.EndRequest(&ctx) }.unwrap();
            let replyvw = MessageViewer::new(reply);

            // TODO: adjust content
            let header_ret = replyvw.get_header();
            assert_eq!(header_ret, String::from("hello: myheader").as_bytes());

            let body_ret = replyvw.get_body();
            assert_eq!(body_ret, String::from("hello: mybody").as_bytes());
        }

        // close client
        {
            let (callback, rx) = AwaitableCallback::create();
            let ctx = unsafe { client.BeginClose(1000, &callback) }.unwrap();
            rx.await.unwrap();
            unsafe { client.EndClose(&ctx) }.unwrap();
        }
    }
}
