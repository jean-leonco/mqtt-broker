use core::fmt;
use std::io::Read;

use anyhow::Context;

// TODO: There should be 2 consts: one for maximum packet size and other for maximum remaining length.
// The maximum packet size is the total number of bytes in an MQTT Control Packet: fixed header size + remaining length size.
// The maximum remaining length is the total number of bytes in an MQTT Control Packet after the fixed header: variable header + payload.
/// MQTT maximum allowed length
pub(crate) const MAX_ALLOWED_LENGTH: usize = 268_435_455;

/// Maximum allowed length for a UTF-8 encoded string
pub(crate) const MAX_STRING_LENGTH: usize = 65_535;

/// Represents the MQTT Control Packet Types.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PacketType {
    /// Connection request.
    /// Sent by: Client to Server.
    Connect = 0x01,

    /// Connect acknowledgment.
    /// Sent by: Server to Client.
    ConnAck = 0x02,

    /// Publish message.
    /// Sent by: Client to Server or Server to Client.
    Publish = 0x03,

    /// Publish acknowledgment (QoS 1).
    /// Sent by: Client to Server or Server to Client.
    PubAck = 0x04,

    /// Publish received (QoS 2 delivery part 1).
    /// Sent by: Client to Server or Server to Client.
    PubRec = 0x05,

    /// Publish release (QoS 2 delivery part 2).
    /// Sent by: Client to Server or Server to Client.
    PubRel = 0x06,

    /// Publish complete (QoS 2 delivery part 3).
    /// Sent by: Client to Server or Server to Client.
    PubComp = 0x07,

    /// Subscribe request.
    /// Sent by: Client to Server.
    Subscribe = 0x08,

    /// Subscribe acknowledgment.
    /// Sent by: Server to Client.
    SubAck = 0x09,

    /// Unsubscribe request.
    /// Sent by: Client to Server.
    Unsubscribe = 0x0A,

    /// Unsubscribe acknowledgment.
    /// Sent by: Server to Client.
    UnsubAck = 0x0B,

    /// PING request.
    /// Sent by: Client to Server.
    PingReq = 0x0C,

    /// PING response.
    /// Sent by: Server to Client.
    PingResp = 0x0D,

    /// Disconnect notification.
    /// Sent by: Client to Server or Server to Client.
    Disconnect = 0x0E,

    /// Authentication exchange.
    /// Sent by: Client to Server or Server to Client.
    Auth = 0x0F,
}

impl PacketType {
    /// Converts a numeric value to an `PacketType`.
    ///
    /// Returns `None` if the value does not match a known type.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Connect),
            0x02 => Some(Self::ConnAck),
            0x03 => Some(Self::Publish),
            0x04 => Some(Self::PubAck),
            0x05 => Some(Self::PubRec),
            0x06 => Some(Self::PubRel),
            0x07 => Some(Self::PubComp),
            0x08 => Some(Self::Subscribe),
            0x09 => Some(Self::SubAck),
            0x0A => Some(Self::Unsubscribe),
            0x0B => Some(Self::UnsubAck),
            0x0C => Some(Self::PingReq),
            0x0D => Some(Self::PingResp),
            0x0E => Some(Self::Disconnect),
            0x0F => Some(Self::Auth),
            _ => None,
        }
    }

    /// Converts the `PacketType` to its numeric value.
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }

    /// Computes the control byte for the MQTT fixed header.
    ///
    /// The control byte is the first byte of the MQTT fixed header, consisting of:
    /// - The packet type (4 most significant bits)
    /// - Packet flags (4 least significant bits)
    ///
    /// This function handles static flags for most packet types as per the MQTT 5.0 specification.
    /// For the `Publish` packet type, dynamic flags such as QoS, retain, and DUP are excluded,
    /// as they must be handled separately.
    ///
    /// # Fixed Header Format
    ///
    /// | Bit       | 7   | 6   | 5   | 4   | 3   | 2   | 1   | 0   |
    /// |-----------|-----|-----|-----|-----|-----|-----|-----|-----|
    /// | Byte 1    | Packet type           | Packet flags          |
    /// | Byte 2    | Remaining Length                              |
    pub fn control_byte(&self) -> u8 {
        let control_byte = match self {
            // For these packets, the 4 LSB are reserved and must be: 0000
            Self::Connect
            | Self::ConnAck
            | Self::PubAck
            | Self::PubRec
            | Self::PubComp
            | Self::SubAck
            | Self::UnsubAck
            | Self::PingReq
            | Self::PingResp
            | Self::Disconnect
            | Self::Auth
            | Self::Publish => {
                // Shift packet type to MSB. For example, with Connect it becomes: 0000_0001 -> 0001_0000
                // Publish allows dynamic flags, but this function does not handle them
                self.to_u8() << 4
            }

            // For these packets, the 4 LSB are reserved and must be: 0010
            Self::PubRel | Self::Subscribe | Self::Unsubscribe => {
                // Shift packet type to MSB and mask it. For example, with PubRel it becomes: 0000_0110 -> 0110_0010
                self.to_u8() << 4 | 0b0000_0010
            }
        };

        control_byte
    }
}

impl fmt::Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Connect => "Connect",
            Self::ConnAck => "ConnAck",
            Self::Publish => "Publish",
            Self::PubAck => "PubAck",
            Self::PubRec => "PubRec",
            Self::PubRel => "PubRel",
            Self::PubComp => "PubComp",
            Self::Subscribe => "Subscribe",
            Self::SubAck => "SubAck",
            Self::Unsubscribe => "Unsubscribe",
            Self::UnsubAck => "UnsubAck",
            Self::PingReq => "PingReq",
            Self::PingResp => "PingResp",
            Self::Disconnect => "Disconnect",
            Self::Auth => "Auth",
        };

        write!(f, "{value}")
    }
}

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
