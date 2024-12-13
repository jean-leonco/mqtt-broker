use std::io::Read;

use anyhow::Context;

// MQTT Control Packet type. Reference: https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901022
pub(crate) const CONNECT: u8 = 1;
pub(crate) const CONNACK: u8 = 2;
// pub(crate) const PUBLISH: u8 = 3;
// pub(crate) const PUBACK: u8 = 4;
// pub(crate) const PUBREC: u8 = 5;
// pub(crate) const PUBREL: u8 = 6;
// pub(crate) const PUBCOMP: u8 = 7;
// pub(crate) const SUBSCRIBE: u8 = 8;
// pub(crate) const SUBACK: u8 = 9;
// pub(crate) const UNSUBSCRIBE: u8 = 10;
// pub(crate) const UNSUBACK: u8 = 11;
// pub(crate) const PINGREQ: u8 = 12;
// pub(crate) const PINGRESP: u8 = 13;
pub(crate) const DISCONNECT: u8 = 14;
pub(crate) const AUTH: u8 = 15;

// Validate the size against the MQTT maximum allowed value
pub(crate) const MAX_ALLOWED_LENGTH: usize = 268_435_455;

/// Decode a variable byte integer
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
pub(crate) fn decode_variable_byte_int(buf: &mut impl Read) -> anyhow::Result<usize> {
    let mut decoded_value = 0usize;
    let mut multiplier = 1;
    let mut encoded_byte = [0; 1];
    let mut encoded_bytes_count = 0;

    loop {
        buf.read_exact(&mut encoded_byte).context("Failed to read encoded byte")?;
        encoded_bytes_count += 1;

        // Prevent excessive continuation bytes (MQTT spec limit)
        if encoded_bytes_count > 4 {
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
