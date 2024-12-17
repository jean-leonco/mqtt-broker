use std::io::Read;

use anyhow::Context;

use super::MAX_STRING_LENGTH;

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
pub(crate) fn decode_variable_byte_int<R: Read>(reader: &mut R) -> anyhow::Result<usize> {
    let mut multiplier = 1;
    let mut decoded_value = 0;
    let mut encoded_byte = [0; 1];

    loop {
        // Read next byte
        reader.read_exact(&mut encoded_byte).context("Failed to read encoded byte")?;

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
pub(crate) fn decode_utf8_string<R: Read>(reader: &mut R) -> anyhow::Result<String> {
    let len = decode_u16(reader).context("Failed to decode string length")? as usize;

    // Allocate a buffer with the exact size needed for the string
    let mut encoded_value = vec![0; len];
    reader
        .read_exact(&mut encoded_value)
        .with_context(|| format!("Failed to read UTF-8 encoded string with length {len}"))?;

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
            "Expected value {value} length ({len}) to be less than or equal to {MAX_STRING_LENGTH}",
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

/// Validates a UTF-8 string based on MQTT protocol requirements.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901010>
///
/// # Errors
/// - Returns an error if:
///   - The string length is bigger than `MAX_STRING_LENGTH`.
///   - The string character data include encodings of code points between U+D800 and U+DFFF.
///   - The string includes an encoding of the null character U+0000.
///   - The string includes U+0001..U+001F control characters, U+007F..U+009F control characters or code points defined in the Unicode specification to be non-characters.
///
pub(crate) fn validate_utf8_string(value: &str) -> anyhow::Result<()> {
    // Unless stated otherwise all UTF-8 encoded strings can have any length in the range 0 to 65,535 bytes
    let len = value.len();
    if value.len() > MAX_STRING_LENGTH {
        anyhow::bail!(
            "String length {len} exceeds maximum allowed size of {MAX_STRING_LENGTH} bytes"
        );
    }

    // Missing: U+D800..U+DFFF, code points defined in the Unicode specification to be non-characters
    if value.chars().all(char::is_control) {
        anyhow::bail!("String include invalid characters");
    }

    Ok(())
}

/// Decode binary data.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901012>
///
/// # Errors
/// - Returns an error if:
///   - Reading the length bytes or binary data bytes fails.
pub(crate) fn decode_binary_data<R: Read>(reader: &mut R) -> anyhow::Result<Vec<u8>> {
    let len = decode_u16(reader).context("Failed to read binary data length")? as usize;

    // Read binary data
    let mut decoded_value = vec![0; len];
    reader
        .read_exact(&mut decoded_value)
        .with_context(|| format!("Failed to read binary data with length {len}"))?;

    Ok(decoded_value)
}

/// Decode a 1-byte unsigned integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901007>
///
/// # Errors
/// - Returns an error if:
///   - Reading the u8 fails.
pub(crate) fn decode_u8<R: Read>(reader: &mut R) -> anyhow::Result<u8> {
    let mut decoded_value = [0; 1];
    reader.read_exact(&mut decoded_value).context("Failed to read u8")?;

    Ok(decoded_value[0])
}

/// Decode a 2-byte unsigned integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901008>
///
/// # Errors
/// - Returns an error if:
///   - Reading the u16 fails.
pub(crate) fn decode_u16<R: Read>(reader: &mut R) -> anyhow::Result<u16> {
    let mut decoded_value = [0; 2];
    reader.read_exact(&mut decoded_value).context("Failed to read u16")?;

    Ok(u16::from_be_bytes(decoded_value))
}

/// Encode a UTF-8 String Pair.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901013>
///
/// # Errors
/// - Returns an error if:
///   - Encoding name fails.
///   - Encoding value fails.
pub(crate) fn encode_utf8_string_pair(
    (name, value): (&String, &String),
) -> anyhow::Result<Vec<u8>> {
    // UTF-8 String Pair are composed by:
    // - 2-byte name length
    // - name
    // - 2-byte value length
    // - value

    let mut encoded_value =
        encode_utf8_string(name).with_context(|| format!("Failed to encode name {name}"))?;

    let mut value =
        encode_utf8_string(value).with_context(|| format!("Failed to encode value {value}"))?;
    encoded_value.append(&mut value);

    Ok(encoded_value)
}
