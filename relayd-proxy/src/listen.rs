use std::net::SocketAddr;

#[derive(Clone, Copy, Debug)]
pub struct Listen(pub SocketAddr);
