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

// TODO: There should be 2 consts: one for maximum packet size and other for maximum remaining length.
// The maximum packet size is the total number of bytes in an MQTT Control Packet: fixed header size + remaining length size.
// The maximum remaining length is the total number of bytes in an MQTT Control Packet after the fixed header: variable header + payload.
/// Validate the size against the MQTT maximum allowed value
pub(crate) const MAX_ALLOWED_LENGTH: usize = 268_435_455;

pub(crate) const MAX_STRING_SIZE: usize = 65_535;

/// Decode a variable byte integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
///
/// Spec implementation:
/// ```text
/// multiplier = 1
/// value = 0
/// do
///    encodedByte = 'next byte from stream'
///    value += (encodedByte AND 127) * multiplier
///    if (multiplier > 128*128*128)
///       throw Error(Malformed Variable Byte Integer)
///    multiplier *= 128
/// while ((encodedByte AND 128) != 0)
/// ```
///
/// where AND is the bit-wise and operator (& in C).
pub(crate) fn decode_variable_byte_int(stream: &mut impl Read) -> anyhow::Result<usize> {
    let mut multiplier = 1;
    let mut decoded_value = 0;
    let mut encoded_byte = [0; 1];

    loop {
        // Read next byte from stream
        stream.read_exact(&mut encoded_byte).context("Failed to read encoded byte")?;

        // Take the 7 least significant bits
        let value = (encoded_byte[0] & 127) as usize;

        // Multiply by current multiplier and add to decoded value
        decoded_value += value * multiplier;

        // Prevent integer overflow
        if multiplier > 128 * 128 * 128 {
            anyhow::bail!("Malformed Variable Byte Integer");
        }

        // Increase multiplier (128, 128^2, 128^3, 128^4)
        multiplier *= 128;

        // Check if continuation bit is not set
        if encoded_byte[0] & 128 == 0 {
            break;
        }
    }

    Ok(decoded_value)
}

/// Encode a variable byte integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
///
/// Spec implementation:
/// ```text
/// do
///    encodedByte = X MOD 128
///    X = X DIV 128
///    // if there are more data to encode, set the top bit of this byte
///    if (X > 0)
///       encodedByte = encodedByte OR 128
///    endif
///    'output' encodedByte
/// while (X > 0)
/// ```
///
/// Where MOD is the modulo operator (% in C), DIV is integer division (/ in C), and OR is bit-wise or (| in C).
pub(crate) fn encode_variable_byte_int(mut value: u32) -> Vec<u8> {
    let capacity = match value {
        0..=127 => 1,
        128..=16_383 => 2,
        16_384..=2_097_151 => 3,
        _ => 4,
    };
    let mut encoded_value = Vec::with_capacity(capacity);

    for _ in 0..capacity {
        // Extract the 7 least significant bits from the current value
        // These 7 bits represent the payload of the current byte
        let mut encoded_byte = (value % 128) as u8;

        // Divide the value by 128 to remove the 7 bits just processed
        // The remaining bits will be processed in the next iteration
        value /= 128;

        // If there are still remaining bits to encode, set MSB to 1
        // This marks the byte as a "continuation byte"
        if value > 0 {
            encoded_byte |= 128;
        }

        encoded_value.push(encoded_byte);
    }

    encoded_value
}

/// Decode a UTF-8 string.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010>
///
/// # Errors
/// - Returns an error if:
///   - Reading the length bytes or string bytes fails.
///   - The string bytes cannot be converted into a valid UTF-8 `String`.
pub(crate) fn decode_utf8_string(stream: &mut impl Read) -> anyhow::Result<String> {
    // Read the 2-byte length prefix, representing the string's length in big-endian format
    let mut encoded_len = [0; 2];
    stream.read_exact(&mut encoded_len).context("Failed to read 2-byte length from stream")?;
    let len = u16::from_be_bytes(encoded_len) as usize;

    // Allocate a buffer with the exact size needed for the string
    let mut encoded_value = vec![0; len];
    stream.read_exact(&mut encoded_value).with_context(|| {
        format!("Failed to read {len} bytes of UTF-8 encoded string from the stream")
    })?;

    // Convert the UTF-8 bytes into a String
    let decoded_value = String::from_utf8(encoded_value)
        .context("Failed to convert encoded_value into a String")?;

    Ok(decoded_value)
}

/// Encode a UTF-8 string.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010>
///
/// # Errors
/// - Returns an error if the string's length cannot be cast to a 16-bit integer.
pub(crate) fn encode_utf8_string(value: &str) -> anyhow::Result<Vec<u8>> {
    // MQTT requires that the length of the string must fit within 2 bytes (0 to 65_535).
    let len = value.len();
    let casted_len = u16::try_from(len).with_context(|| {
        format!(
            "Expected value {value} length ({len}) to be less than or equal to {MAX_STRING_SIZE}",
        )
    })?;

    // Preallocate a buffer with sufficient capacity:
    // - 2 bytes for the length field
    // - `len` bytes for the UTF-8 encoded string
    let mut encoded_value = Vec::with_capacity(2 + len);

    // Big-endian ensures compatibility with the MQTT specification
    encoded_value.extend(casted_len.to_be_bytes());

    // Append the UTF-8 encoded bytes of the string to the buffer
    encoded_value.extend(value.as_bytes());

    Ok(encoded_value)
}
