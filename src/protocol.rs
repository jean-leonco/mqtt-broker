use std::{error::Error, fmt};

pub(crate) mod decoding;
pub(crate) mod encoding;
pub(crate) mod packet_type;

/// Protocol name.
pub(crate) const PROTOCOL_NAME: &str = "MQTT";

/// Protocol version.
pub(crate) const PROTOCOL_VERSION: u8 = 5;

/// Maximum allowed size for a packet.
pub(crate) const MAX_PACKET_SIZE: usize = 268_435_455;

/// Maximum allowed length for a UTF-8 encoded string.
pub(crate) const MAX_STRING_LENGTH: usize = 65_535;

/// Data within the packet could not be correctly parsed.
#[derive(Debug)]
pub struct MalformedPacketError(pub String);

impl fmt::Display for MalformedPacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Malformed packet: {}", self.0)
    }
}

impl Error for MalformedPacketError {}
