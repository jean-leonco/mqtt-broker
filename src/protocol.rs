use std::error::Error;
use std::fmt;

pub(crate) mod mqtt_data;
pub(crate) mod packet_type;

/// Protocol name.
pub(crate) const PROTOCOL_NAME: &str = "MQTT";

/// Protocol version.
pub(crate) const PROTOCOL_VERSION: u8 = 5;

/// Maximum allowed size for a packet.
pub(crate) const MAX_PACKET_SIZE: usize = 268_435_455;

/// Maximum allowed length for a UTF-8 encoded string.
pub(crate) const MAX_STRING_LENGTH: usize = 65_535;
