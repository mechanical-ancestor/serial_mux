use std::{io, ops::Deref, path::PathBuf};

use anyhow::Context;
use tokio::net::UnixDatagram;

pub struct Socket {
    pub path: PathBuf,
    pub socket: UnixDatagram,
}

impl Deref for Socket {
    type Target = UnixDatagram;

    fn deref(&self) -> &Self::Target {
        &self.socket
    }
}

impl Socket {
    pub fn new(socket_path: PathBuf) -> Result<Self, anyhow::Error> {
        let socket = UnixDatagram::unbound().context("Failed to create datagram socket")?;
        Ok(Self {
            path: socket_path,
            socket,
        })
    }

    pub async fn send(&self, data: &[u8]) -> Result<usize, io::Error> {
        self.send_to(data, &self.path).await
    }
}
