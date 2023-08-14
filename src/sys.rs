// low level constructs for SF

use std::cell::RefCell;

use service_fabric_rs::FabricCommon::{
    FabricTransport::{
        IFabricTransportMessage, IFabricTransportMessage_Impl, FABRIC_TRANSPORT_MESSAGE_BUFFER,
    },
    IFabricAsyncOperationCallback, IFabricAsyncOperationCallback_Impl,
    IFabricAsyncOperationContext, IFabricAsyncOperationContext_Impl, IFabricStringResult,
};

use tokio::sync::oneshot::{self, Receiver, Sender};
use windows::core::{implement, AsImpl, HSTRING};

#[allow(non_snake_case)]
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

// view IFabricString
pub struct StringViewer {
    s: IFabricStringResult,
}

impl StringViewer {
    pub fn new(s: IFabricStringResult) -> StringViewer {
        StringViewer { s }
    }

    pub fn get_hstring(&self) -> HSTRING {
        let pwstr = unsafe { self.s.get_String() };
        let buff = unsafe { pwstr.as_wide() };
        // Hstring takes ownership
        HSTRING::from_wide(buff)
    }
}

#[cfg(test)]
mod tests {

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
        assert!(cast.IsCompleted().as_bool());
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
}
