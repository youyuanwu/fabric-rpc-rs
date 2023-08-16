use std::{collections::HashMap, ffi::c_void, sync::Mutex};

use service_fabric_rs::{
    FabricCommon::{
        FabricTransport::{
            CreateFabricTransportListener, IFabricTransportClientConnection,
            IFabricTransportConnectionHandler, IFabricTransportConnectionHandler_Impl,
            IFabricTransportListener, IFabricTransportMessage, IFabricTransportMessageDisposer,
            IFabricTransportMessageHandler, IFabricTransportMessageHandler_Impl,
            FABRIC_TRANSPORT_LISTEN_ADDRESS, FABRIC_TRANSPORT_SETTINGS,
        },
        IFabricAsyncOperationCallback, IFabricAsyncOperationContext,
        IFabricAsyncOperationContext_Impl,
    },
    FABRIC_E_NOT_READY,
};
use tokio::sync::mpsc::{self, Receiver, Sender};
//use tokio::sync::Mutex;
use windows::{
    core::{implement, Error, Interface, HRESULT, HSTRING},
    Win32::Foundation::E_POINTER,
};

use crate::{
    shared_tr::MsgDispoer,
    sys::{raw_to_hstring, AwaitableCallback, Context, ContextWrapper, StringViewer},
};

// server code
//#[derive(Debug)]
#[implement(IFabricTransportConnectionHandler)]
struct ServerConnHandler {
    internal: *mut c_void, // need to use cvoid because there is no way to anotate lifetime for implements
}

impl ServerConnHandler {
    pub fn new(internal: *mut c_void) -> ServerConnHandler {
        ServerConnHandler { internal }
    }

    #[allow(clippy::mut_from_ref)]
    fn get_internal(&self) -> &mut ServerInternal {
        let cast = self.internal as *mut ServerInternal;
        unsafe { &mut *cast }
    }
}

#[allow(non_snake_case)]
impl IFabricTransportConnectionHandler_Impl for ServerConnHandler {
    fn BeginProcessConnect(
        &self,
        clientconnection: &::core::option::Option<IFabricTransportClientConnection>,
        _timeoutmilliseconds: u32,
        callback: &::core::option::Option<IFabricAsyncOperationCallback>,
    ) -> ::windows::core::Result<IFabricAsyncOperationContext> {
        if callback.is_none() || clientconnection.is_none() {
            return Err(E_POINTER.into());
        }

        let cb = callback.clone().unwrap();
        // push the connection
        let client = clientconnection.clone().unwrap();
        self.get_internal().push(client)?;

        let mut ctx = Context::new(cb.clone());
        ctx.complete();
        unsafe { cb.Invoke(&ctx.clone().into()) };
        Ok(ctx.into())
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
        clientid: *const u16,
        _timeoutmilliseconds: u32,
        callback: &::core::option::Option<IFabricAsyncOperationCallback>,
    ) -> ::windows::core::Result<IFabricAsyncOperationContext> {
        if callback.is_none() {
            return Err(E_POINTER.into());
        }

        let cb = callback.clone().unwrap();
        let id = raw_to_hstring(clientid);

        self.get_internal().disconnect(id);

        let mut ctx = Context::new(cb.clone());
        ctx.complete();
        unsafe { cb.Invoke(&ctx.clone().into()) };
        Ok(ctx.into())
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

//#[derive(Debug)]
#[implement(IFabricTransportMessageHandler)]
struct MessageHandler {
    internal: *const c_void,
}

impl MessageHandler {
    // internal is the ServerInternal
    pub fn new(internal: *const c_void) -> MessageHandler {
        MessageHandler { internal }
    }

    // this is to satisfy the com implements functions are immutable
    #[allow(clippy::mut_from_ref)]
    fn get_internal(&self) -> &mut ServerInternal {
        let cast = self.internal as *mut ServerInternal;
        unsafe { &mut *cast }
    }
}

#[allow(non_snake_case)]
impl IFabricTransportMessageHandler_Impl for MessageHandler {
    fn BeginProcessRequest(
        &self,
        clientid: *const u16,
        message: &::core::option::Option<IFabricTransportMessage>,
        _timeoutmilliseconds: u32,
        callback: &::core::option::Option<IFabricAsyncOperationCallback>,
    ) -> ::windows::core::Result<IFabricAsyncOperationContext> {
        if message.is_none() || callback.is_none() {
            return Err(E_POINTER.into());
        }

        let msg = message.clone().unwrap();
        let cb = callback.clone().unwrap();

        let ctx = Context::new(cb);

        let id = raw_to_hstring(clientid);
        let req = ServerRequest {
            msg,
            ctx: ctx.clone(),
        };
        self.get_internal().push_requst(id, req)?;

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

// TODO: split the server connection into internal entry and reveiver end.
// server internal. keeps track of connections
#[derive(Debug)]
struct ServerInternal {
    conns: Mutex<HashMap<String, ServerConnectionInternal>>,
    rx: Receiver<ServerConnection>,
    tx: Sender<ServerConnection>,
}

unsafe impl Send for ServerInternal {}
unsafe impl Sync for ServerInternal {}

impl ServerInternal {
    pub fn new() -> ServerInternal {
        let (tx, rx) = mpsc::channel::<ServerConnection>(100);
        ServerInternal {
            conns: Mutex::new(HashMap::new()),
            tx,
            rx,
        }
    }

    // add a connection
    pub fn push(&mut self, client: IFabricTransportClientConnection) -> Result<(), Error> {
        let id_raw = unsafe { client.get_ClientId() };
        // hstring does not have hash impl
        let id = raw_to_hstring(id_raw).to_string();

        let (tx, rx) = tokio::sync::mpsc::channel::<ServerRequest>(100);
        let conn = ServerConnection::new(client, rx);
        let conn_internal = ServerConnectionInternal::new(tx);

        let res = self.tx.blocking_send(conn);
        if res.is_err() {
            // TODO: remove connection?
            let err = res.err().unwrap();
            let ret_err = Error::new(
                HRESULT(FABRIC_E_NOT_READY.0),
                HSTRING::from(err.to_string()),
            );
            return Err(ret_err);
        }

        self.conns.lock().unwrap().insert(id, conn_internal);
        Ok(())
    }

    pub async fn async_pop(&mut self) -> ServerConnection {
        self.rx.recv().await.unwrap()
    }

    // pub fn get_receiver(&self) -> &Receiver<Arc<Mutex<ServerConnection>>> {
    //     &self.rx
    // }

    pub fn disconnect(&mut self, id: HSTRING) {
        let val = self.conns.lock().unwrap().remove(&id.to_string());
        if let Some(mut vv) = val {
            vv.disconnected = true;
        } else {
            panic!("disconnect of non exist connection");
        }
    }

    // push a msg to a connection
    pub fn push_requst(&mut self, id: HSTRING, req: ServerRequest) -> Result<(), Error> {
        let cc = self.conns.lock().unwrap();
        let val = cc.get(&id.to_string());
        if let Some(vv) = val {
            vv.push(req)?;
        } else {
            panic!("request pushed to unknown connection");
        }
        Ok(())
    }
}

#[derive(Debug)]
// request item that server needs to process
pub struct ServerRequest {
    msg: IFabricTransportMessage,
    ctx: Context, // context returned to FabricTransport
}

unsafe impl Send for ServerRequest {}
unsafe impl Sync for ServerRequest {}

impl ServerRequest {
    pub fn new(msg: IFabricTransportMessage, ctx: Context) -> ServerRequest {
        ServerRequest { msg, ctx }
    }

    pub fn complete(&mut self, reply: IFabricTransportMessage) {
        self.ctx.set_msg(reply);
        self.ctx.complete();

        // notify the reply is ready
        let cb = self.ctx.Callback().unwrap();
        unsafe { cb.Invoke(&self.ctx.clone().into()) };
    }

    pub fn get_request_msg(&self) -> &IFabricTransportMessage {
        &self.msg
    }
}

#[derive(Debug)]
pub struct ServerConnection {
    rx: Receiver<ServerRequest>,
    // can be used to send back msg
    _client: IFabricTransportClientConnection,
}

unsafe impl Send for ServerConnection {}
unsafe impl Sync for ServerConnection {}

impl ServerConnection {
    pub fn new(
        client: IFabricTransportClientConnection,
        rx: Receiver<ServerRequest>,
    ) -> ServerConnection {
        ServerConnection {
            rx,
            _client: client,
        }
    }

    pub async fn async_accept(&mut self) -> ServerRequest {
        // if rx is not closed there is always item to pop
        self.rx.recv().await.unwrap()
    }
}

// internal book keeping for server connection
#[derive(Debug)]
struct ServerConnectionInternal {
    tx: Sender<ServerRequest>,
    disconnected: bool, // TODO: share this with public
}

impl ServerConnectionInternal {
    fn new(tx: Sender<ServerRequest>) -> ServerConnectionInternal {
        ServerConnectionInternal {
            tx,
            disconnected: false,
        }
    }

    // transport can sync push into the queue
    pub fn push(&self, req: ServerRequest) -> Result<(), Error> {
        let res = self.tx.blocking_send(req);
        if res.is_ok() {
            Ok(())
        } else {
            let err = res.err().unwrap();
            let ret_err = Error::new(
                HRESULT(FABRIC_E_NOT_READY.0),
                HSTRING::from(err.to_string()),
            );
            Err(ret_err)
        }
    }
}

// server
pub struct ServerTransport {
    l: IFabricTransportListener,
    internal: Box<ServerInternal>,
}

unsafe impl Send for ServerTransport {}
unsafe impl Sync for ServerTransport {}

impl ServerTransport {
    pub fn new(
        settings: &FABRIC_TRANSPORT_SETTINGS,
        address: &FABRIC_TRANSPORT_LISTEN_ADDRESS,
    ) -> Result<ServerTransport, Error> {
        // get the raw addr of the internal to be shared
        let internal = Box::new(ServerInternal::new());
        let raw_internal = &*internal as *const ServerInternal;
        let raw_mut_internal = raw_internal as *mut ServerInternal;
        let raw_void_internal = raw_mut_internal as *mut c_void;

        let disposeprocessor: IFabricTransportMessageDisposer = MsgDispoer::new().into();
        let svr_conn_handler: IFabricTransportConnectionHandler =
            ServerConnHandler::new(raw_void_internal).into();
        let msg_handler: IFabricTransportMessageHandler =
            MessageHandler::new(raw_void_internal).into();
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
        Ok(ServerTransport {
            l: listener,
            internal,
        })
    }

    pub async fn open(&self) -> Result<HSTRING, Error> {
        let ctx_wapper: ContextWrapper;
        let rxx: tokio::sync::oneshot::Receiver<()>;
        {
            let (callback, rx) = AwaitableCallback::create();
            let ctx = unsafe { self.l.BeginOpen(&callback) }?;
            ctx_wapper = ContextWrapper::new(ctx);
            rxx = rx;
        }
        rxx.await.unwrap();
        let addr = unsafe { self.l.EndOpen(&ctx_wapper.get()) }?;
        let sv = StringViewer::new(addr);
        Ok(sv.get_hstring())
    }

    pub async fn close(&self) -> Result<(), Error> {
        let ctx_wapper: ContextWrapper;
        let rxx: tokio::sync::oneshot::Receiver<()>;
        {
            let (callback, rx) = AwaitableCallback::create();
            let ctx = unsafe { self.l.BeginClose(&callback) }?;
            ctx_wapper = ContextWrapper::new(ctx);
            rxx = rx;
        }
        rxx.await.unwrap();
        unsafe { self.l.EndClose(&ctx_wapper.get()) }?;

        Ok(())
    }

    pub async fn async_accept(&mut self) -> ServerConnection {
        self.internal.async_pop().await
    }
}
