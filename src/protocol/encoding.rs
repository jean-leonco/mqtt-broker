use anyhow::Context;

/// Encode a variable byte integer.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>
///
/// **Specification:**
///
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
/// where MOD is the modulo operator (% in C), DIV is integer division (/ in C), and OR is bit-wise or (| in C).
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
///
/// **Specification:**
/// - Length must fit in a 16-bit unsigned integer.
/// - The string must be valid UTF-8.
///
/// # Errors
/// - Returns an error if the string length exceeds the maximum allowed.
pub(crate) fn encode_utf8_string(value: &str) -> anyhow::Result<Vec<u8>> {
    // MQTT requires that the length of the string must fit within 2 bytes (0 to 65_535).
    let len = value.len();
    let casted_len = u16::try_from(len).context("String length exceeds the maximum allowed")?;

    // Allocate a buffer with sufficient capacity:
    // - 2 bytes for the length field
    // - `len` bytes for the UTF-8 encoded string
    let mut encoded_value = Vec::with_capacity(2 + len);
    encoded_value.extend(casted_len.to_be_bytes());
    encoded_value.extend(value.as_bytes());

    Ok(encoded_value)
}

/// Encode a UTF-8 String Pair.
///
/// Reference: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901013>
///
/// **Specification:**
/// - Consists of a 2-byte length + string, followed by another 2-byte length + string.
///
/// # Errors
/// - Returns an error if encoding either name or value fails.
pub(crate) fn encode_utf8_string_pair(
    (name, value): (&String, &String),
) -> anyhow::Result<Vec<u8>> {
    let mut name = encode_utf8_string(name)
        .with_context(|| format!("Failed to encode UTF-8 string name {name}"))?;

    let mut value = encode_utf8_string(value)
        .with_context(|| format!("Failed to encode UTF-8 string value {value}"))?;

    name.append(&mut value);

    Ok(name)
}
