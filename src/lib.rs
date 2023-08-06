pub fn add(left: usize, right: usize) -> usize {
    left + right
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
        IFabricTransportMessage, IFabricTransportMessageDisposer, FABRIC_TRANSPORT_SETTINGS,
    };
    use service_fabric_rs::{
        FabricCommon::FabricTransport::{
            IFabricTransportClient, IFabricTransportClientEventHandler_Impl,
            IFabricTransportMessageDisposer_Impl,
        },
        FABRIC_SECURITY_CREDENTIALS, FABRIC_SECURITY_CREDENTIAL_KIND_NONE,
    };
    use windows::core::{implement, Interface, HSTRING};

    use super::*;

    #[derive(Debug)]
    #[implement(IFabricTransportCallbackMessageHandler)]
    pub struct ClientMsgHandler {}

    impl ClientMsgHandler {
        pub fn new() -> ClientMsgHandler {
            ClientMsgHandler {}
        }
    }

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

    impl IFabricTransportMessageDisposer_Impl for MsgDispoer {
        fn Dispose(
            &self,
            _count: u32,
            _messages: *const ::core::option::Option<IFabricTransportMessage>,
        ) {
            ()
        }
    }

    #[test]
    fn fabric_transport() {
        let mut creds = FABRIC_SECURITY_CREDENTIALS::default();
        creds.Kind = FABRIC_SECURITY_CREDENTIAL_KIND_NONE;
        let mut settings = FABRIC_TRANSPORT_SETTINGS::default();
        settings.KeepAliveTimeoutInSeconds = 10;
        settings.MaxConcurrentCalls = 10;
        settings.MaxMessageSize = 10;
        settings.MaxQueueSize = 10;
        settings.OperationTimeoutInSeconds = 10;
        settings.SecurityCredentials = &creds;

        let connectionaddress = HSTRING::from("localhost:12345+/");
        let notificationhandler: IFabricTransportCallbackMessageHandler =
            ClientMsgHandler::new().into();
        let clienteventhandler: IFabricTransportClientEventHandler = ClientEvHandler::new().into();
        let messagedisposer: IFabricTransportMessageDisposer = MsgDispoer::new().into();

        let _client = unsafe {
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
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
