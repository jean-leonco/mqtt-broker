use std::{fmt, io::Read};

use bytes::{Buf, BytesMut};
use log::debug;

use crate::{
    codec::{decode_utf8_string, decode_variable_byte_int},
    constants::PUBLISH_PACKET_TYPE,
};

use super::{CommonPacketError, DecodablePacket};

#[derive(Debug)]
pub(crate) struct PublishPacket {
    pub(crate) topic: String,
    pub(crate) payload: Option<BytesMut>,
}

#[derive(Debug)]
pub(crate) enum PublishPacketDecodeError {
    Common(CommonPacketError),
}

impl std::error::Error for PublishPacketDecodeError {}

impl fmt::Display for PublishPacketDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Common(e) => write!(f, "Common Error: {e}"),
        }
    }
}

impl DecodablePacket for PublishPacket {
    type Error = PublishPacketDecodeError;

    fn packet_type() -> u8 {
        PUBLISH_PACKET_TYPE
    }

    fn validate_header(fixed_header: u8) -> Result<(), Self::Error> {
        let packet_type = fixed_header >> 4;
        if packet_type != Self::packet_type() {
            let e = CommonPacketError::MalformedPacket(Some(format!(
                "Invalid packet type: {packet_type}"
            )));
            return Err(Self::Error::Common(e));
        }

        Ok(())
    }

    fn decode(
        cursor: &mut std::io::Cursor<&[u8]>,
        _fixed_header: u8,
        remaining_len: usize,
    ) -> Result<Self, Self::Error> {
        let start = cursor.position();

        // TODO: Validate topic name
        let topic = decode_utf8_string(cursor).map_err(Self::Error::Common)?;

        let properties_len = decode_variable_byte_int(cursor).map_err(Self::Error::Common)?;
        cursor.advance(properties_len);
        debug!("properties_len: {properties_len}");

        let payload_len = remaining_len - (cursor.position() - start) as usize;
        debug!("payload_len: {payload_len}");

        if payload_len > 0 {
            let mut buf = vec![0; payload_len];
            cursor
                .read_exact(&mut buf)
                .map_err(|_| Self::Error::Common(CommonPacketError::UnexpectedError))?;

            Ok(Self { topic, payload: Some(BytesMut::from(&buf[..])) })
        } else {
            Ok(Self { topic, payload: None })
        }
    }
}
