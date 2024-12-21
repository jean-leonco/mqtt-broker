use std::fmt;

use crate::constants::PINGREQ_PACKET_TYPE;

use super::{CommonPacketError, DecodablePacket};

#[derive(Debug)]
pub(crate) struct PingReqPacket {}

#[derive(Debug)]
pub(crate) enum PingReqPacketDecodeError {
    Common(CommonPacketError),
}

impl std::error::Error for PingReqPacketDecodeError {}

impl fmt::Display for PingReqPacketDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Common(e) => write!(f, "Common Error: {e}"),
        }
    }
}

impl DecodablePacket for PingReqPacket {
    type Error = PingReqPacketDecodeError;

    fn packet_type() -> u8 {
        PINGREQ_PACKET_TYPE
    }

    fn validate_header(fixed_header: u8) -> Result<(), Self::Error> {
        let packet_type = fixed_header >> 4;
        if packet_type != Self::packet_type() {
            let e = CommonPacketError::MalformedPacket(Some(format!(
                "Invalid packet type: {packet_type}"
            )));
            return Err(Self::Error::Common(e));
        }

        // Reserved flags (4 LSB) must be equal to 0000
        let flags = fixed_header << 4;
        if flags != 0x0 {
            let e =
                CommonPacketError::MalformedPacket(Some("Fixed header flags are reserved".into()));
            return Err(Self::Error::Common(e));
        }

        Ok(())
    }

    fn decode(_cursor: &mut std::io::Cursor<&[u8]>) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}
