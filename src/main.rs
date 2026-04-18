use std::io;

use anyhow::Context;
use clap::arg;
use futures_lite::{FutureExt, StreamExt};
use tokio::{io::AsyncWriteExt as _, net::UnixDatagram};

mod config;
mod serial;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let mut args = clap::Command::new("serial_mux")
        .arg(arg!(-c --config <CONFIG> "Config file to use, default to ./config.toml"))
        .get_matches();
    let config_path = args
        .remove_one::<String>("config")
        .unwrap_or_else(|| "config.toml".into());

    let config = config::Config::new(config_path).context("Failed to load config")?;

    let (upstreams, mut downstream) = serial::new(&config)?;

    if config.bind_socket.exists() {
        std::fs::remove_file(&config.bind_socket)?;
    }

    let unix_socket = UnixDatagram::bind(&config.bind_socket)?;

    upstreams
        // Thanks to UnixDatagram only requires &self to send data,
        // this async closure implements FnMut.
        .then(async |packet| {
            let target = config.routes.iter().find_map(|route| {
                (route.upstream.header == packet.header).then_some(&route.socket_path)
            })?;
            unix_socket
                .send_to(&packet.data, target)
                .await
                .map_err(|e| match e.kind() {
                    io::ErrorKind::NotFound => log::warn!("{} was not found", target.display()),
                    _ => log::error!("Failed to send packet: {e}"),
                })
                .ok()
        })
        .for_each(drop)
        .or(async {
            // Just forward all received packets to the downstream serial port.
            let mut buf = [0; 1024];
            while let Ok(len) = unix_socket
                .recv(&mut buf)
                .await
                .map_err(|e| log::error!("Failed to receive packet: {e}"))
            {
                downstream
                    .write_all(&buf[..len])
                    .await
                    .map_err(|e| log::error!("Failed to write to serial: {e}"))
                    .ok();
            }
        })
        .await;

    Ok(())
}
