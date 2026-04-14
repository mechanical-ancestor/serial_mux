use anyhow::Context;
use clap::arg;
use futures_lite::StreamExt;
use tokio::net::UnixDatagram;

mod config;
mod serial;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut args = clap::Command::new("serial_mux")
        .arg(arg!(-c --config <CONFIG> "Optionally sets a config file to use"))
        .get_matches();
    let config_path = args
        .remove_one::<String>("config")
        .unwrap_or_else(|| "config.toml".into());

    let config = config::Config::new(config_path).context("Failed to load config")?;

    let upstreams = serial::new(&config)?;

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
                .map_err(|e| log::error!("Failed to send packet: {}", e))
                .ok()
        })
        .for_each(drop)
        .await;

    Ok(())
}
