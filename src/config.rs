use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use serde::Deserialize;

use crate::serial::{CRCAlgorithm, Header};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub serial: SerialConfig,
    pub bind_socket: PathBuf,
    pub routes: Vec<Route>,
}

impl Config {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config_file = fs::read_to_string(path.as_ref()).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => {
                anyhow!("Config file: {} was not found", path.as_ref().display())
            }
            _ => From::from(e),
        })?;
        toml::from_str::<Self>(&config_file).map_err(From::from)
    }
}

#[derive(Debug, Deserialize)]
pub struct SerialConfig {
    pub dev_path: String,
    pub baud_rate: u32,
}

#[derive(Debug, Deserialize)]
pub struct Route {
    pub socket_path: PathBuf,
    pub upstream: SerialPacketConfig,
    #[expect(unused)]
    pub downstream: Option<PacketConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SerialPacketConfig {
    pub header: Header,
    pub data_len: usize,
    pub crc: Option<CRCAlgorithm>,
}

#[expect(unused)]
#[derive(Debug, Deserialize)]
pub struct PacketConfig {
    pub header: Header,
    pub data_len: usize,
}
