// client transport

use service_fabric_rs::FabricCommon::FabricTransport::{
    CreateFabricTransportClient, IFabricTransportCallbackMessageHandler,
    IFabricTransportCallbackMessageHandler_Impl, IFabricTransportClient,
    IFabricTransportClientEventHandler, IFabricTransportClientEventHandler_Impl,
    IFabricTransportMessage, IFabricTransportMessageDisposer, FABRIC_TRANSPORT_SETTINGS,
};
use windows::core::{implement, Error, Interface, HSTRING};

use crate::{shared_tr::MsgDispoer, sys::AwaitableCallback};

// required COM obj for client
#[derive(Debug)]
#[implement(IFabricTransportCallbackMessageHandler)]
pub struct ClientMsgHandler {}

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
pub struct ClientEvHandler {}

impl ClientEvHandler {
    pub fn new() -> ClientEvHandler {
        ClientEvHandler {}
    }
}

impl Default for ClientEvHandler {
    fn default() -> Self {
        Self::new()
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

// client object
pub struct ClientTransport {
    c: IFabricTransportClient,
}

impl ClientTransport {
    pub fn new(
        settings: &FABRIC_TRANSPORT_SETTINGS,
        connectionaddress: &HSTRING,
    ) -> Result<ClientTransport, Error> {
        let notificationhandler: IFabricTransportCallbackMessageHandler =
            ClientMsgHandler::new().into();
        let clienteventhandler: IFabricTransportClientEventHandler = ClientEvHandler::new().into();
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
        Ok(ClientTransport { c: client })
    }

    pub async fn open(&self) -> Result<(), Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.c.BeginOpen(1000, &callback) }?;
        rx.await.unwrap();
        unsafe { self.c.EndOpen(&ctx) }?;
        Ok(())
    }

    pub async fn request(
        &self,
        msg: &IFabricTransportMessage,
    ) -> Result<IFabricTransportMessage, Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.c.BeginRequest(msg, 1000, &callback) }?;
        rx.await.unwrap();
        let reply = unsafe { self.c.EndRequest(&ctx) }?;
        Ok(reply)
    }

    pub async fn close(&self) -> Result<(), Error> {
        let (callback, rx) = AwaitableCallback::create();
        let ctx = unsafe { self.c.BeginClose(1000, &callback) }?;
        rx.await.unwrap();
        unsafe { self.c.EndClose(&ctx) }?;
        Ok(())
    }
}
