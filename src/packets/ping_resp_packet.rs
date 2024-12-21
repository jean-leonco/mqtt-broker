use std::fmt;

use bytes::BytesMut;

use crate::constants::PINGRESP_PACKET_TYPE;

use super::EncodablePacket;

#[derive(Debug)]
pub(crate) struct PingRespPacket {}

#[derive(Debug)]
pub(crate) struct PingRespPacketEncodeError {}

impl std::error::Error for PingRespPacketEncodeError {}

impl fmt::Display for PingRespPacketEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PingRespPacketEncodeError")
    }
}

impl EncodablePacket for PingRespPacket {
    type Error = PingRespPacketEncodeError;

    fn encode(&self) -> Result<bytes::BytesMut, Self::Error> {
        Ok(BytesMut::from(&[PINGRESP_PACKET_TYPE << 4, 0][..]))
    }
}
