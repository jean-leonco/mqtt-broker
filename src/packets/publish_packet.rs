use std::{fmt, io::Read};

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{
    codec::{
        decode_utf8_string, decode_variable_byte_int, write_usize_as_var_int, write_utf8_string,
    },
    constants::{MAX_PACKET_SIZE, PUBLISH_PACKET_TYPE},
};

use super::{CommonPacketError, DecodablePacket, EncodablePacket, Packet};

#[derive(Debug)]
pub(crate) struct PublishPacket {
    pub(crate) topic: String,
    pub(crate) payload: Option<Bytes>,
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

impl PublishPacket {
    pub(crate) fn new(topic: String, payload: Option<Bytes>) -> Self {
        Self { topic, payload }
    }
}

impl Packet for PublishPacket {
    fn packet_type() -> u8 {
        PUBLISH_PACKET_TYPE
    }
}

impl DecodablePacket for PublishPacket {
    type Error = PublishPacketDecodeError;

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

        let payload_len = remaining_len - (cursor.position() - start) as usize;
        if payload_len > 0 {
            let mut buf = vec![0; payload_len];
            cursor
                .read_exact(&mut buf)
                .map_err(|_| Self::Error::Common(CommonPacketError::UnexpectedError))?;

            Ok(Self { topic, payload: Some(Bytes::from(buf)) })
        } else {
            Ok(Self { topic, payload: None })
        }
    }
}

impl EncodablePacket for PublishPacket {
    type Error = PublishPacketDecodeError;

    fn encode(&self) -> Result<bytes::BytesMut, Self::Error> {
        // topic name
        let mut variable_header_buf = BytesMut::new();
        write_utf8_string(&mut variable_header_buf, &self.topic).map_err(Self::Error::Common)?;

        let mut buf = BytesMut::new();

        // Fixed header
        buf.put_u8(Self::packet_type() << 4);

        let payload_len = match &self.payload {
            Some(payload) => payload.len(),
            _ => 0,
        };

        // Remaining length: topic + properties len + payload
        let remaining_len = variable_header_buf.len() + 1 + payload_len;
        write_usize_as_var_int(&mut buf, remaining_len).map_err(Self::Error::Common)?;

        buf.put(&variable_header_buf[..]);

        // properties len
        buf.put_u8(0x0);

        // payload
        if let Some(payload) = &self.payload {
            buf.put(&payload[..])
        }

        if buf.len() > MAX_PACKET_SIZE {
            return Err(Self::Error::Common(CommonPacketError::PacketTooLarge(None)));
        }

        Ok(buf)
    }
}
