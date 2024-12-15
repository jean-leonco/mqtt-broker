use std::{
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

pub(crate) mod packets;
pub(crate) mod protocol;

use anyhow::{Context, Ok};
use log::{debug, error, info, trace};
use packets::{connect_packet::ConnectPacket, disconnect_packet};
use protocol::PacketType;

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
    trace!("Starting to handle new client connection");

    let mut writer = stream.try_clone().context("Failed to clone writer stream")?;
    let mut reader = BufReader::new(stream);

    // MQTT protocol operates by exchanging control packets
    // The packet is composed by:
    // - Fixed header (1 byte) containing:
    //   - The packet type represented by a 4 bit uint
    //   - Packet flags specific to the packet type. If it's marked as reserved, it could be used in the future and thus it can't be ignored by the server (FOR NOW)
    // - Remaining length (1-4 bytes). Contains how many bytes are in the variable header (if exists) and payload
    // - Variable header (variable size). Content varies depending on the packet type
    // - Payload (variable size). Required for some packet types like connect and publish

    // TODO: Handle Malformed packet: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#S4_13_Errors
    let mut fixed_header = [0; 1];
    reader.read_exact(&mut fixed_header).context("Failed to read fixed header")?;
    let packet_type = fixed_header[0] >> 4;

    debug!("Received packet_type: {packet_type}");

    match PacketType::from_u8(packet_type) {
        Some(packet_type) => match packet_type {
            PacketType::Connect => {
                let packet = ConnectPacket::decode(&mut reader)
                    .context("Failed to decode Connect packet")?;
                debug!("Connect packet: {:?}", packet);

                let response = disconnect_packet::DisconnectPacket::new(
                disconnect_packet::DisconnectReasonCode::NotAuthorized,
                None,
                Some(String::from(
                    "Client is not authorized to perform this action. Please verify credentials.",
                )),
                None,
                None,
            )
            .context("Failed to create DISCONNECT packet")?
            .encode()
            .context("Failed to encode DISCONNECT packet")?;

                info!("Sending DISCONNECT packet (NotAuthorized) to the client");
                writer.write_all(&response).context("Failed to write DISCONNECT packet")?;
            }
            // Packet type not implemented
            _ => anyhow::bail!("Packet type {} not implemented", packet_type),
        },
        // Packet types outside expected range
        _ => anyhow::bail!("Packet type {} invalid", packet_type),
    }

    info!("Connection handled successfully");
    Ok(())
}
