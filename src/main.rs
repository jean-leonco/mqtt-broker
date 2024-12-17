use std::{
    io::BufReader,
    net::{TcpListener, TcpStream},
};

pub(crate) mod handlers;
pub(crate) mod packets;
pub(crate) mod protocol;

use anyhow::{Context, Ok};
use handlers::connect_handler;
use log::{debug, error, info};
use protocol::{decoding, packet_type::PacketType};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    info!("Starting MQTT broker on 127.0.0.1:1883...");
    let listener = TcpListener::bind("127.0.0.1:1883").context("Failed to bind on port 1883")?;
    info!("Broker is listening on 127.0.0.1:1883");

    loop {
        let (stream, addr) = listener.accept().context("Failed to accept incoming connection")?;
        info!("Client connected: {}", addr);

        // Handle the client connection in a separate function for clarity.
        if let Err(e) = handle_connection(stream) {
            error!("Error handling connection from {}: {:?}", addr, e);
        }
    }
}

/// Handle a single MQTT client connection.
///
/// # Errors
/// Returns an error if the packet is malformed or processing fails.
fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    debug!("Starting to handle new client connection");

    let mut writer = stream.try_clone().context("Failed to clone writer stream")?;
    let mut reader = BufReader::new(stream);

    let fixed_header = decoding::decode_u8(&mut reader).context("Failed to read fixed header")?;

    let packet_type = fixed_header >> 4;
    debug!("Received packet_type: {packet_type}");

    // TODO: Handle Malformed packet: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#S4_13_Errors
    match PacketType::from_u8(packet_type) {
        Some(packet_type) => match packet_type {
            PacketType::Connect => connect_handler::handle_connect(&mut reader, &mut writer)?,
            // Packet type not implemented
            _ => anyhow::bail!("Packet type {} not implemented", packet_type),
        },
        // Packet types outside expected range
        _ => anyhow::bail!("Packet type {} invalid", packet_type),
    }

    info!("Connection handled successfully");
    Ok(())
}
