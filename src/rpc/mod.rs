pub mod api;
pub mod handler;

use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub enum RpcStatus {
    Disconnected,
    Connecting,
    Connected,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct RpcClient {
    addr: SocketAddr,
    status: RpcStatus,
}

impl RpcClient {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            status: RpcStatus::Disconnected,
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn status(&self) -> &RpcStatus {
        &self.status
    }

    pub fn mark_connected(&mut self) {
        self.status = RpcStatus::Connected;
    }
}
