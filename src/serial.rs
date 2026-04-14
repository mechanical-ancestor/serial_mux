use anyhow::Context;
use bytes::{Buf as _, Bytes, BytesMut};
use futures_lite::{Stream, StreamExt};
use serde::Deserialize;
use tokio_serial::SerialPortBuilderExt as _;
use tokio_util::codec::{self, Framed};

use crate::config::{Config, SerialPacketConfig};

pub const HEADER_SIZE: usize = 2;
pub type Header = [u8; HEADER_SIZE];

pub fn new(config: &Config) -> anyhow::Result<impl Stream<Item = SerialPacket>> {
    let serial = tokio_serial::new(&config.serial.dev_path, config.serial.baud_rate)
        .open_native_async()
        .context("Failed to open serial port")?;

    let upstreams = config
        .routes
        .iter()
        .map(|route| route.upstream.clone())
        // Codec need to access all upstreams.
        .collect::<Vec<_>>();

    let serial = Framed::new(serial, SerialCodec(upstreams))
        .map_while(Result::ok)
        .inspect(|packet| log::debug!("{:x?}", &packet.data));

    Ok(serial)
}

#[cfg(feature = "crc")]
use crc::Crc;

#[cfg(feature = "crc")]
const CRC16: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_CMS);

#[cfg(feature = "crc")]
const CRC32: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_ISCSI);

#[derive(Debug)]
pub struct SerialPacket {
    pub header: Header,
    pub data: Bytes,
}

impl SerialPacketConfig {
    const fn packet_len(&self) -> usize {
        HEADER_SIZE + self.data_len + self.crc.len()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum CRCAlgorithm {
    CRC16,
    CRC32,
}

impl CRCAlgorithm {
    const fn len(&self) -> usize {
        match self {
            CRCAlgorithm::CRC16 => 2,
            CRCAlgorithm::CRC32 => 4,
        }
    }
}

pub struct SerialCodec(
    // Why not HashMap? Because we probably have several routes,
    // it's not worth to use HashMap.
    Vec<SerialPacketConfig>,
);

impl codec::Decoder for SerialCodec {
    type Item = SerialPacket;

    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        log::trace!("buffer capacity: {}", src.capacity());

        let Some(header) = src.first_chunk::<HEADER_SIZE>() else {
            log::trace!("No header found in buffer");
            src.reserve(HEADER_SIZE);
            return Ok(None);
        };

        let Some(config) = self.0.iter().find(|config| &config.header == header) else {
            log::trace!("No config found for header: {header:?}");
            src.advance(HEADER_SIZE);
            src.reserve(HEADER_SIZE);
            return Ok(None);
        };

        let packet_len = config.packet_len();

        if src.len() < packet_len {
            src.reserve(packet_len - src.len());
            return Ok(None);
        }
        let header = *header;

        let mut packet = src.split_to(packet_len).freeze();

        #[allow(unused_variables)]
        let crc_bytes = packet.split_off(packet_len - config.crc.len());

        #[cfg(feature = "crc")]
        {
            let mut crc_bytes = crc_bytes;

            let is_crc_valid = match config.crc {
                CRCAlgorithm::CRC16 => crc_bytes.get_u16() == CRC16.checksum(&packet),
                CRCAlgorithm::CRC32 => crc_bytes.get_u32() == CRC32.checksum(&packet),
            };

            if !is_crc_valid {
                log::error!("CRC check failed for packet: {:x?}", packet);
                src.reserve(packet_len);
                return Ok(None);
            }
        }

        src.reserve(packet_len);
        Ok(Some(SerialPacket {
            header,
            data: packet,
        }))
    }
}
