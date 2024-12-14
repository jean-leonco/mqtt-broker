use core::str;
use std::{
    io::{Cursor, Read, Write},
    net::{TcpListener, TcpStream},
};

pub(crate) mod packets;
pub(crate) mod protocol;

use anyhow::{Context, Ok};
use log::{debug, error, info, trace, warn};
use packets::disconnect_packet;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    info!("Starting MQTT broker on 127.0.0.1:1883...");
    let listener = TcpListener::bind("127.0.0.1:1883").context("Failed to bind on port 1883")?;
    info!("Broker is listening on 127.0.0.1:1883.");

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
fn handle_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    trace!("Starting to handle new client connection.");

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
    stream.read_exact(&mut fixed_header).context("Failed to read fixed header")?;
    let packet_type = fixed_header[0] >> 4;

    debug!("Received packet_type: {packet_type}");

    match protocol::PacketType::from_u8(packet_type) {
        Some(packet_type) => match packet_type {
            protocol::PacketType::Connect => {
                // https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901033
                // TODO: The Server MUST process a second CONNECT packet sent from a Client as a Protocol Error and close the Network Connection

                debug!("Handling CONNECT packet.");

                let remaining_len = protocol::decode_variable_byte_int(&mut stream)
                    .context("Failed to decode remaining length")?;
                debug!("CONNECT packet remaining_len: {}", remaining_len);

                if remaining_len > protocol::MAX_ALLOWED_LENGTH {
                    warn!(
                        "CONNECT packet too large: {} > {}",
                        remaining_len,
                        protocol::MAX_ALLOWED_LENGTH
                    );
                    return Err(anyhow::anyhow!("Packet too large"));
                }

                let mut rest = vec![0; remaining_len];
                stream
                    .read_exact(&mut rest)
                    .context("Failed to read variable header and payload")?;
                let mut rest_buf = Cursor::new(rest);

                // Decode Protocol Name length
                let mut protocol_name_len = [0; 2];
                rest_buf
                    .read_exact(&mut protocol_name_len)
                    .context("Failed to read protocol name len")?;
                let protocol_name_len = u16::from_be_bytes(protocol_name_len);
                if protocol_name_len != 4 {
                    warn!("Unexpected protocol name length: {}", protocol_name_len);
                    return Err(anyhow::anyhow!("Unsupported Protocol Version"));
                }

                // The Protocol Name is a UTF-8 Encoded String that represents the protocol name “MQTT”
                let mut protocol_name = [0; 4];
                rest_buf.read_exact(&mut protocol_name).context("Failed to read protocol name")?;
                let protocol_name =
                    str::from_utf8(&protocol_name).context("Protocol name is not valid UTF-8")?;

                if protocol_name != "MQTT" {
                    warn!("Unsupported protocol name: {}", protocol_name);
                    return Err(anyhow::anyhow!("Unsupported Protocol Version"));
                }
                debug!("Protocol name: {}", protocol_name);

                // The value of the Protocol Version field for version 5.0 of the protocol is 5 (0x05)
                let mut protocol_level = [0; 1];
                rest_buf
                    .read_exact(&mut protocol_level)
                    .context("Failed to read protocol level")?;
                let protocol_level = u8::from_be_bytes(protocol_level);
                if protocol_level != 5 {
                    warn!("Unsupported protocol level: {}", protocol_level);
                    return Err(anyhow::anyhow!("Unsupported Protocol Version"));
                }
                debug!("Protocol level: {}", protocol_level);

                // The Connect Flags byte contains several parameters specifying the behavior of the MQTT connection. It also indicates the presence or absence of fields in the Payload
                // https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901038
                let mut connect_flags_buf = [0; 1];
                rest_buf
                    .read_exact(&mut connect_flags_buf)
                    .context("Failed to read connect flags")?;
                let connect_flags = connect_flags_buf[0];

                // The Server MUST validate that the reserved flag in the CONNECT packet is set to 0
                if connect_flags & 1 != 0 {
                    warn!("Malformed packet: reserved flag set in CONNECT packet.");
                    return Err(anyhow::anyhow!("Malformed packet"));
                }

                // If the username flag is set to 0, a username must not be present in the payload. Otherwise, a username must be present in the payload.
                let username_flag = (connect_flags >> 7) & 1 == 1;

                // If the password flag is set to 0, a password must not be present in the payload. Otherwise, a password must be present in the payload.
                let password_flag = (connect_flags >> 6) & 1 == 1;
                // If the Will Flag is set to 1 and Will Retain is set to 0, the Server MUST publish the Will Message as a non-retained message.
                // If the Will Flag is set to 1 and Will Retain is set to 1, the Server MUST publish the Will Message as a retained message.
                let will_retain = (connect_flags >> 5) & 1 == 1;
                let will_flag = (connect_flags >> 2) & 1 == 1;

                // These two bits specify the QoS level to be used when publishing the Will Message.
                let will_qos = (connect_flags >> 3) & 0b0000_0011;

                let clean_start = (connect_flags >> 1) & 1 == 1;

                // If the Will Flag is set to 0, then Will Retain MUST be set to 0.
                if !will_flag && will_retain {
                    warn!("Malformed packet: will_flag is 0 but will_retain is set.");
                    return Err(anyhow::anyhow!("Malformed packet"));
                }

                // If the Will Flag is set to 0, then the Will QoS MUST be set to 0 (0x00).
                if (!will_flag && will_qos != 0) || will_qos > 2 {
                    warn!("Malformed packet: Invalid will_qos when will_flag is disabled or QoS out of range.");
                    return Err(anyhow::anyhow!("Malformed packet"));
                }

                debug!("Connect flags: username_flag={}, password_flag={}, will_retain={}, will_qos={}, will_flag={}, clean_start={}",
                username_flag, password_flag, will_retain, will_qos, will_flag, clean_start
            );

                // The Keep Alive is a Two Byte Integer which is a time interval measured in seconds. It is the maximum time interval that is permitted to elapse between the point at which the Client finishes transmitting one MQTT Control Packet and the point it starts sending the next. It is the responsibility of the Client to ensure that the interval between MQTT Control Packets being sent does not exceed the Keep Alive value. If Keep Alive is non-zero and in the absence of sending any other MQTT Control Packets, the Client MUST send a PINGREQ packet.
                let mut keep_alive = [0; 2];
                rest_buf.read_exact(&mut keep_alive).context("Failed to read keep alive")?;
                let keep_alive = u16::from_be_bytes(keep_alive);
                debug!("Keep alive: {}", keep_alive);

                // TODO: Decode properties
                let properties_len = protocol::decode_variable_byte_int(&mut rest_buf)
                    .context("Failed to decode properties length")?;
                debug!("properties_len: {}", properties_len);

                // The Payload of the CONNECT packet contains one or more length-prefixed fields, whose presence is determined by the flags in the Variable Header. These fields, if present, MUST appear in the order Client Identifier, Will Properties, Will Topic, Will Payload, User Name, Password.

                // The Client Identifier (ClientID) identifies the Client to the Server. Each Client connecting to the Server has a unique ClientID. The ClientID MUST be used by Clients and by Servers to identify state that they hold relating to this MQTT Session between the Client and the Server.
                // The ClientID MUST be present and is the first field in the CONNECT packet Payload.
                // The ClientID MUST be a UTF-8 Encoded String as defined in section 1.5.4.
                // The Server MUST allow ClientID’s which are between 1 and 23 UTF-8 encoded bytes in length, and that contain only the characters "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".
                // The Server MAY allow ClientID’s that contain more than 23 encoded bytes. The Server MAY allow ClientID’s that contain characters not included in the list given above.
                // A Server MAY allow a Client to supply a ClientID that has a length of zero bytes, however if it does so the Server MUST treat this as a special case and assign a unique ClientID to that Client. It MUST then process the CONNECT packet as if the Client had provided that unique ClientID, and MUST return the Assigned Client Identifier in the CONNACK packet.
                // If the Server rejects the ClientID it MAY respond to the CONNECT packet with a CONNACK using Reason Code 0x85 (Client Identifier not valid) as described in section 4.13 Handling errors, and then it MUST close the Network Connection.

                // No clientId
                //if rest_buf.len() == 0 {
                //    anyhow::bail!("Client Identifier not valid");
                //}

                // protocol_name_length + protocol_name + protocol_level + connect_flags + keep_alive
                let variable_header_len = 2 + 4 + 1 + 1 + 2 + properties_len;
                let payload_len = remaining_len - variable_header_len;

                // Reserve capacity for the payload vector
                let mut buf = Vec::with_capacity(payload_len);
                rest_buf.read_to_end(&mut buf).context("Failed to read payload")?;

                trace!("Raw payload (hex): {}", hex::encode(&buf));
                debug!("Payload length: {}", payload_len);
                debug!("Expected client_id: {}", hex::encode("clientIdW7T3SRR5d3"));
                debug!("Expected client_id_len: {}", "clientIdW7T3SRR5d3".len());
                debug!("Expected username: {}", hex::encode("123"));
                debug!("Expected password: {}", hex::encode("123123"));

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

                info!("Sending DISCONNECT packet (NotAuthorized) to the client.");
                stream.write_all(&response).context("Failed to write DISCONNECT packet")?;
            }
            // Packet type not implemented
            _ => anyhow::bail!("Packet type {} not implemented", packet_type),
        },
        // Packet types outside expected range
        _ => anyhow::bail!("Packet type {} invalid", packet_type),
    }

    info!("Connection handled successfully.");
    Ok(())
}
