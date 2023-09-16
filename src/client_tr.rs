// client transport

use std::cell::RefCell;

use service_fabric_rs::FabricCommon::FabricTransport::{
    CreateFabricTransportClient, IFabricTransportCallbackMessageHandler,
    IFabricTransportCallbackMessageHandler_Impl, IFabricTransportClient,
    IFabricTransportClientEventHandler, IFabricTransportClientEventHandler_Impl,
    IFabricTransportMessage, IFabricTransportMessageDisposer, FABRIC_TRANSPORT_SETTINGS,
};
use tokio::sync::oneshot::{self, Receiver, Sender};
use windows::core::{implement, Error, Interface, HRESULT, HSTRING};

use crate::{shared_tr::MsgDispoer, sys::AwaitableCallback};

// required COM obj for client
#[derive(Debug)]
#[implement(IFabricTransportCallbackMessageHandler)]
struct ClientMsgHandler {}

impl ClientMsgHandler {
    pub fn new() -> ClientMsgHandler {
        ClientMsgHandler {}
    }
}

impl Default for ClientMsgHandler {
    fn default() -> Self {
        Self::new()
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
struct ClientEvHandler {
    conn_tx: RefCell<Option<Sender<()>>>,
    disconn_tx: RefCell<Option<Sender<HRESULT>>>,
}

impl ClientEvHandler {
    pub fn new(conn_tx: Sender<()>, disconn_tx: Sender<HRESULT>) -> ClientEvHandler {
        ClientEvHandler {
            conn_tx: RefCell::new(Some(conn_tx)),
            disconn_tx: RefCell::new(Some(disconn_tx)),
        }
    }
}

#[allow(non_snake_case)]
impl IFabricTransportClientEventHandler_Impl for ClientEvHandler {
    fn OnConnected(
        &self,
        _connectionaddress: &::windows::core::PCWSTR,
    ) -> ::windows::core::Result<()> {
        let tx = self.conn_tx.take();
        if let Some(txx) = tx {
            txx.send(()).unwrap();
        } else {
            panic!("Connect can only happen once")
        }
        Ok(())
    }

    fn OnDisconnected(
        &self,
        _connectionaddress: &::windows::core::PCWSTR,
        error: ::windows::core::HRESULT,
    ) -> ::windows::core::Result<()> {
        let tx = self.disconn_tx.take();
        if let Some(txx) = tx {
            txx.send(error).unwrap();
            println!("Client event disconnected");
        } else {
            panic!("Disconnect can only happen once")
        }
        Ok(())
    }
}

// client object
pub struct ClientTransport {
    c: IFabricTransportClient,
    conn_rx: Option<Receiver<()>>,
    disconn_rx: Option<Receiver<HRESULT>>,
}

impl ClientTransport {
    pub fn new(
        settings: &FABRIC_TRANSPORT_SETTINGS,
        connectionaddress: &HSTRING,
    ) -> Result<ClientTransport, Error> {
        let (conn_tx, conn_rx) = oneshot::channel::<()>();
        let (disconn_tx, disconn_rx) = oneshot::channel::<HRESULT>();

        let notificationhandler: IFabricTransportCallbackMessageHandler =
            ClientMsgHandler::new().into();
        let clienteventhandler: IFabricTransportClientEventHandler =
            ClientEvHandler::new(conn_tx, disconn_tx).into();
        let messagedisposer: IFabricTransportMessageDisposer = MsgDispoer::new().into();

        let client = unsafe {
            CreateFabricTransportClient(
                &IFabricTransportClient::IID,
                settings,
                connectionaddress,
                &notificationhandler,
                &clienteventhandler,
                &messagedisposer,
            )?
        };

        Ok(ClientTransport {
            c: client,
            conn_rx: Some(conn_rx),
            disconn_rx: Some(disconn_rx),
        })
    }

    // wait for connection
    pub async fn connect(&mut self) {
        let rx = self.conn_rx.take();
        if let Some(rxx) = rx {
            rxx.await.unwrap();
        } else {
            panic!("Connect can only happen once")
        }
    }

    // Note if the client is closed. This may be stuck forever.
    // This waits for server to drop connection.
    // returns the hr for why disconnection happened
    pub async fn disconnect(&mut self) -> HRESULT {
        let rx: Option<Receiver<HRESULT>> = self.disconn_rx.take();
        if let Some(rxx) = rx {
            rxx.await.unwrap()
        } else {
            panic!("Disconnect can only happen once")
        }
    }

    pub async fn open(&self, timoutmilliseconds: u32) -> Result<(), Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.c.BeginOpen(timoutmilliseconds, &callback) }?;
        rx.await.unwrap();
        unsafe { self.c.EndOpen(&ctx) }?;
        Ok(())
    }

    pub async fn request(
        &self,
        timoutmilliseconds: u32,
        msg: &IFabricTransportMessage,
    ) -> Result<IFabricTransportMessage, Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.c.BeginRequest(msg, timoutmilliseconds, &callback) }?;
        rx.await.unwrap();
        let reply = unsafe { self.c.EndRequest(&ctx) }?;
        Ok(reply)
    }

    pub async fn close(&self, timoutmilliseconds: u32) -> Result<(), Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.c.BeginClose(timoutmilliseconds, &callback) }?;
        rx.await.unwrap();
        unsafe { self.c.EndClose(&ctx) }?;
        // TODO: maybe trigger the disconnect signal here to be safe.
        Ok(())
    }
}
