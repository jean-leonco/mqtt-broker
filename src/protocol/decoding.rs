use std::io::Read;

use super::MalformedPacketError;

/// Decode a variable byte integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
///
/// **Specification:**
///
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
///
/// # Errors
/// - Returns `CommonDecodeError::MalformedPacket` if reading from the stream fails or if the variable byte integer is malformed.
pub(crate) fn decode_variable_byte_int<R: Read>(
    reader: &mut R,
) -> Result<usize, MalformedPacketError> {
    let mut multiplier = 1;
    let mut decoded_value = 0;
    let mut encoded_byte = [0; 1];

    loop {
        // Read next byte
        reader.read_exact(&mut encoded_byte).map_err(|_| {
            MalformedPacketError("Unable to read variable byte integer".to_string())
        })?;

        // Take the 7 least significant bits
        let value = (encoded_byte[0] & 127) as usize;
        // Multiply by current multiplier and add to decoded value
        decoded_value += value * multiplier;

        // Ensure multiplier does not exceed the specification limits
        if multiplier > 128 * 128 * 128 {
            return Err(MalformedPacketError("Malformed variable byte integer".to_string()));
        }

        multiplier *= 128;

        // If the continuation bit is not set, we are done
        if encoded_byte[0] & 128 == 0 {
            break;
        }
    }

    Ok(decoded_value)
}

/// Decode a UTF-8 string.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010>
///
/// **Specification:**
/// - Length is a two-byte integer representing the size of the following string.
/// - String must be valid UTF-8.
///
/// # Errors
/// - Returns `CommonDecodeError::MalformedPacket` if reading fails or string is invalid UTF-8.
pub(crate) fn decode_utf8_string<R: Read>(reader: &mut R) -> Result<String, MalformedPacketError> {
    let len = decode_u16(reader)
        .map_err(|_| MalformedPacketError("Failed to read UTF-8 string length".to_string()))?
        as usize;

    let mut encoded_value = vec![0; len];
    reader
        .read_exact(&mut encoded_value)
        .map_err(|_| MalformedPacketError("Failed to read UTF-8 string data".to_string()))?;

    String::from_utf8(encoded_value)
        .map_err(|_| MalformedPacketError("String is not valid UTF-8".to_string()))
}

/// Decode binary data.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901012>
///
/// **Specification:**
/// - The length is a two-byte integer followed by that many bytes of data.
///
/// # Errors
/// - Returns `CommonDecodeError::MalformedPacket` if reading fails.
pub(crate) fn decode_binary_data<R: Read>(reader: &mut R) -> Result<Vec<u8>, MalformedPacketError> {
    let len = decode_u16(reader)
        .map_err(|_| MalformedPacketError("Failed to read binary data length".to_string()))?
        as usize;

    let mut decoded_value = vec![0; len];
    reader
        .read_exact(&mut decoded_value)
        .map_err(|_| MalformedPacketError("Failed to read binary data".to_string()))?;

    Ok(decoded_value)
}

/// Decode a 1-byte unsigned integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901007>
///
/// # Errors
/// - Returns `CommonDecodeError::MalformedPacket` if reading fails.
pub(crate) fn decode_u8<R: Read>(reader: &mut R) -> Result<u8, MalformedPacketError> {
    let mut decoded_value = [0; 1];
    reader
        .read_exact(&mut decoded_value)
        .map_err(|_| MalformedPacketError("Failed to read u8".to_string()))?;

    Ok(decoded_value[0])
}

/// Decode a 2-byte unsigned integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901008>
///
/// # Errors
/// - Returns `CommonDecodeError::MalformedPacket` if reading fails.
pub(crate) fn decode_u16<R: Read>(reader: &mut R) -> Result<u16, MalformedPacketError> {
    let mut decoded_value = [0; 2];
    reader
        .read_exact(&mut decoded_value)
        .map_err(|_| MalformedPacketError("Failed to read u16".to_string()))?;

    Ok(u16::from_be_bytes(decoded_value))
}
