// shared transport stuff

use fabric_base::FabricCommon::FabricTransport::{
    IFabricTransportMessage, IFabricTransportMessageDisposer, IFabricTransportMessageDisposer_Impl,
};
use windows::core::implement;

#[derive(Debug)]
#[implement(IFabricTransportMessageDisposer)]
pub struct MsgDispoer {}

impl MsgDispoer {
    pub fn new() -> MsgDispoer {
        MsgDispoer {}
    }
}

impl Default for MsgDispoer {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(non_snake_case)]
impl IFabricTransportMessageDisposer_Impl for MsgDispoer {
    fn Dispose(
        &self,
        _count: u32,
        _messages: *const ::core::option::Option<IFabricTransportMessage>,
    ) {
    }
}
