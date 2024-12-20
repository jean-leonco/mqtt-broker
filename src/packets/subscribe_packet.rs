use std::io::Cursor;

use bytes::Buf;
use log::debug;

use crate::{
    codec::{decode_utf8_string, decode_variable_byte_int},
    connection::PacketError,
};

#[derive(Debug)]
pub(crate) enum RetainHandling {
    SendAllAtSubscribe,
    SendIfNewSubscription,
    DoNotSendAtSubscribe,
}

impl RetainHandling {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::SendAllAtSubscribe),
            1 => Some(Self::SendIfNewSubscription),
            2 => Some(Self::DoNotSendAtSubscribe),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TopicFilter {
    pub name: String,
    qos_level: u8,
    retain_as_published: bool,
    retain_handling: RetainHandling,
}

#[derive(Debug)]
pub(crate) struct SubscribePacket {
    pub topics: Vec<TopicFilter>,
}

#[derive(Debug)]
pub(crate) enum DecodeError {
    PacketError(PacketError),
    ProtocolError,
}

impl SubscribePacket {
    pub fn decode(buf: &mut Cursor<&[u8]>) -> Result<Self, DecodeError> {
        let packet_id = buf.get_u16();
        debug!("packet_id: {}", packet_id);

        let properties_len = decode_variable_byte_int(buf).map_err(DecodeError::PacketError)?;
        buf.advance(properties_len);

        if !buf.has_remaining() {
            return Err(DecodeError::ProtocolError);
        }

        let mut topics = Vec::new();
        while buf.has_remaining() {
            let name = decode_utf8_string(buf).map_err(DecodeError::PacketError)?;

            let subscription_options = buf.get_u8();

            let qos_level = subscription_options & 0b0000_0011;
            if qos_level > 2 {
                return Err(DecodeError::ProtocolError);
            }

            let retain_as_published = subscription_options >> 3 & 1 == 1;

            let retain_handling =
                match RetainHandling::from_u8(subscription_options >> 4 & 0b0000_0011) {
                    Some(value) => value,
                    _ => return Err(DecodeError::ProtocolError),
                };

            let reserved_bits = subscription_options >> 6 & 0b0000_0011;
            if reserved_bits != 0x0 {
                return Err(DecodeError::PacketError(PacketError::MalformedPacket));
            }
            topics.push(TopicFilter { name, qos_level, retain_as_published, retain_handling });
        }

        Ok(Self { topics })
    }
}
