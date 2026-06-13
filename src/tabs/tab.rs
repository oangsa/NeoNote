use uuid::Uuid;

use crate::{
    embed::{hwnd::WindowHandle, NeovideInstance},
    rpc::RpcClient,
};

pub struct Tab {
    pub id: Uuid,
    pub title: String,
    pub modified: bool,
    pub nvim_instance: NeovideInstance,
    pub rpc: RpcClient,
    pub hwnd: WindowHandle,
}
