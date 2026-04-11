use anyhow::Context;
use clap::arg;
use futures_lite::StreamExt;

mod config;
mod serial;
mod socket;

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

    let unix_sockets = config
        .routes
        .iter()
        .map(|route| {
            Ok((
                route.upstream.header,
                socket::Socket::new(route.socket_path.clone())?,
            ))
        })
        // Why not HashMap? Because we probably have several routes,
        // it's not worth to use HashMap.
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    upstreams
        // Thanks to UnixDatagram only requires &self to send data,
        // this async closure implements FnMut.
        .then(async |packet| {
            unix_sockets
                .iter()
                .find_map(|(header, socket)| (header == &packet.header).then_some(socket))?
                .send(&packet.data)
                .await
                .map_err(|e| log::error!("Failed to send packet: {}", e))
                .ok()
        })
        .for_each(drop)
        .await;

    Ok(())
}
