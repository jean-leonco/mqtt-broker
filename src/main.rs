use core::str;
use std::{
    io::{Cursor, Read},
    net::{TcpListener, TcpStream},
};

use anyhow::Context;

// MQTT Control Packet type. Reference: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901022
const CONNECT: u8 = 1;
const CONNACK: u8 = 2;
//const PUBLISH: u8 = 3;
//const PUBACK: u8 = 4;
//const PUBREC: u8 = 5;
//const PUBREL: u8 = 6;
//const PUBCOMP: u8 = 7;
//const SUBSCRIBE: u8 = 8;
//const SUBACK: u8 = 9;
//const UNSUBSCRIBE: u8 = 10;
//const UNSUBACK: u8 = 11;
//const PINGREQ: u8 = 12;
//const PINGRESP: u8 = 13;
//const DISCONNECT: u8 = 14;
const AUTH: u8 = 15;

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1883").context("Failed to bind 1883 port")?;

    for stream in listener.incoming() {
        let stream = stream.context("Failed to accept connection")?;
        handle_client(stream)?;
    }

    Ok(())
}

// Reference: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901019
fn handle_client(mut stream: TcpStream) -> anyhow::Result<()> {
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
    match packet_type {
        CONNECT => {
            // https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901033

            let remaining_length =
                decode_variable_integer(&mut stream, 4).context("Failed to remaining length")?;

            // Read rest of the packet
            let mut rest = vec![0; remaining_length];
            stream.read_exact(&mut rest).context("Failed to read variable header and payload")?;
            let mut rest_buf = Cursor::new(rest);

            // After a Network Connection is established by a Client to a Server, the first packet sent from the Client to the Server MUST be a CONNECT packet
            // TODO: The Server MUST process a second CONNECT packet sent from a Client as a Protocol Error and close the Network Connection

            // The Variable Header for the CONNECT Packet contains the following fields in this order: Protocol Name, Protocol Level, Connect Flags, Keep Alive, and Properties

            let mut protocol_name_length = [0; 2];
            rest_buf.read_exact(&mut protocol_name_length).context("")?;
            let protocol_name_length = u16::from_be_bytes(protocol_name_length);
            if protocol_name_length != 4 {
                anyhow::bail!("Unsupported Protocol Version")
            }

            // The Protocol Name is a UTF-8 Encoded String that represents the protocol name “MQTT”
            let mut protocol_name = [0; 4];
            rest_buf.read_exact(&mut protocol_name)?;
            if str::from_utf8(&protocol_name)? != "MQTT" {
                anyhow::bail!("Unsupported Protocol Version")
            }

            // The value of the Protocol Version field for version 5.0 of the protocol is 5 (0x05)
            let mut protocol_level = [0; 1];
            rest_buf.read_exact(&mut protocol_level).context("")?;
            let protocol_level = u8::from_be_bytes(protocol_level);
            if protocol_level != 5 {
                anyhow::bail!("Unsupported Protocol Version")
            }

            // The Connect Flags byte contains several parameters specifying the behavior of the MQTT connection. It also indicates the presence or absence of fields in the Payload
            // https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901038
            let mut connect_flags_buf = [0; 1];
            rest_buf.read_exact(&mut connect_flags_buf).context("")?;
            let connect_flags = connect_flags_buf[0];
            println!("{connect_flags:08b}");

            // The Server MUST validate that the reserved flag in the CONNECT packet is set to 0
            if connect_flags & 1 != 0 {
                anyhow::bail!("Malformed packet");
            };

            // If the username flag is set to 0, a username must not be present in the payload. Otherwise, a username must be present in the payload.
            let username_flag = match (connect_flags >> 7) & 1 {
                1 => Some(""),
                _ => None,
            };

            // If the password flag is set to 0, a password must not be present in the payload. Otherwise, a password must be present in the payload.
            let password_flag = match (connect_flags >> 6) & 1 {
                1 => Some(""),
                _ => None,
            };

            // If the Will Flag is set to 1 and Will Retain is set to 0, the Server MUST publish the Will Message as a non-retained message.
            // If the Will Flag is set to 1 and Will Retain is set to 1, the Server MUST publish the Will Message as a retained message.
            let will_retain = (connect_flags >> 5) & 1 == 1;
            let will_flag = (connect_flags >> 2) & 1 == 1;

            // If the Will Flag is set to 0, then Will Retain MUST be set to 0.
            if !will_flag && will_retain {
                anyhow::bail!("Malformed packet");
            }

            // These two bits specify the QoS level to be used when publishing the Will Message.
            let will_qos = (connect_flags >> 3) & 0b0000_0011;

            // If the Will Flag is set to 0, then the Will QoS MUST be set to 0 (0x00).
            if (!will_flag && will_qos != 0) || will_qos > 2 {
                anyhow::bail!("Malformed packet");
            }

            let clear_start = (connect_flags >> 1) & 1 == 1;

            let mut keep_alive_buf = [0; 2];
            rest_buf.read_exact(&mut keep_alive_buf).context("")?;
            // The Keep Alive is a Two Byte Integer which is a time interval measured in seconds. It is the maximum time interval that is permitted to elapse between the point at which the Client finishes transmitting one MQTT Control Packet and the point it starts sending the next. It is the responsibility of the Client to ensure that the interval between MQTT Control Packets being sent does not exceed the Keep Alive value. If Keep Alive is non-zero and in the absence of sending any other MQTT Control Packets, the Client MUST send a PINGREQ packet [MQTT-3.1.2-20].
            let keep_alive = u16::from_be_bytes(keep_alive_buf);

            println!(
                "username: {username_flag:?}\npassword: {password_flag:?}\nwill_retain: {will_retain}\nwill_qos: {will_qos}\nwill_flag: {will_flag}\nclear_start: {clear_start}\nkeep_alive: {keep_alive}"
            );
        }
        // TODO: Send the error to the client
        CONNACK..AUTH => anyhow::bail!("Packet type {} not implemented", packet_type),
        packet_type => anyhow::bail!("Packet type {} invalid", packet_type),
    }

    Ok(())
}

/// Decode a variable integer from a readable
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
fn decode_variable_integer(buf: &mut impl Read, max: usize) -> anyhow::Result<usize> {
    let mut decoded_value = 0usize;
    let mut multiplier = 1;
    let mut encoded_byte = [0; 1];
    let mut encoded_bytes_count = 0;

    loop {
        buf.read_exact(&mut encoded_byte).context("Failed to encoded byte")?;
        encoded_bytes_count += 1;

        // Prevent excessive continuation bytes (MQTT spec limit)
        if encoded_bytes_count > max {
            anyhow::bail!("Malformed packet");
        }

        // Take the 7 least significant bits
        let value = (encoded_byte[0] & 0x7F) as usize;
        // Multiply by current multiplier and add to remaining length
        decoded_value += value * multiplier;
        // Increase multiplier (128, 128^2, 128^3, etc.)
        multiplier *= 128;

        // Prevent integer overflow
        if multiplier > 128 * 128 * 128 * 128 {
            anyhow::bail!("Value too large");
        }

        // Check if continuation bit is not set
        if encoded_byte[0] & 0x80 == 0 {
            break;
        }
    }

    Ok(decoded_value)
}
