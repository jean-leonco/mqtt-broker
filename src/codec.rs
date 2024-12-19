use std::io::Cursor;

use bytes::{Buf, BytesMut};

use crate::{connection::PacketError, constants::MAX_STRING_LENGTH};

/// Decode a variable byte integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
pub(crate) fn decode_variable_byte_int(buf: &mut Cursor<&[u8]>) -> Result<usize, PacketError> {
    let mut multiplier = 1;
    let mut decoded_value = 0;

    loop {
        let encoded_byte = buf.get_u8();

        // Take the 7 least significant bits
        let value = (encoded_byte & 127) as usize;

        // Multiply by current multiplier and add to decoded value
        decoded_value += value * multiplier;

        // Ensure multiplier does not exceed the specification limits
        if multiplier > 128 * 128 * 128 {
            return Err(PacketError::MalformedPacket);
        }

        multiplier *= 128;

        // If the continuation bit is not set, we are done
        if encoded_byte & 128 == 0 {
            break;
        }
    }

    Ok(decoded_value)
}

/// Decode a UTF-8 string.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010>
pub(crate) fn decode_utf8_string(buf: &mut Cursor<&[u8]>) -> Result<String, PacketError> {
    // Read the length of the string
    let len = buf.get_u16() as usize;

    // Read the string data
    let mut encoded_value = vec![0; len];
    if let Err(_) = std::io::Read::read_exact(buf, &mut encoded_value) {
        return Err(PacketError::UnexpectedError);
    }

    // Convert to UTF-8 string
    match String::from_utf8(encoded_value) {
        Ok(value) => Ok(value),
        Err(_) => Err(PacketError::UnexpectedError),
    }
}

/// Decode a binary buf.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901012>
pub(crate) fn decode_binary_data(buf: &mut Cursor<&[u8]>) -> Result<BytesMut, PacketError> {
    // Read the length of the binary data
    let len = buf.get_u16() as usize;

    // Read the binary data
    let mut decoded_value = BytesMut::with_capacity(len);
    match std::io::Read::read_exact(buf, &mut decoded_value) {
        Ok(_) => Ok(decoded_value),
        Err(_) => Err(PacketError::UnexpectedError),
    }
}

/// Encode a variable byte integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
///
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

        // If there are still remaining bits, mark this byte as continuation
        if value > 0 {
            encoded_byte |= 128;
        }

        encoded_value.push(encoded_byte);
    }

    encoded_value
}

/// Encode a UTF-8 string.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010>
pub(crate) fn encode_utf8_string(value: &str) -> Result<Vec<u8>, PacketError> {
    let len = value.len();

    if len > MAX_STRING_LENGTH {
        return Err(PacketError::PacketTooLarge);
    }

    let mut encoded_value = Vec::with_capacity(2 + len);
    encoded_value.extend(len.to_be_bytes());
    encoded_value.extend(value.as_bytes());

    Ok(encoded_value)
}

/// Encode a UTF-8 String Pair.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901013>
pub(crate) fn encode_utf8_string_pair(
    (name, value): (&String, &String),
) -> Result<Vec<u8>, PacketError> {
    let mut name = encode_utf8_string(name)?;
    let mut value = encode_utf8_string(value)?;

    name.append(&mut value);

    Ok(name)
}
