use std::{fmt, io::Cursor};

use bytes::Buf;

use crate::{
    codec::{decode_utf8_string, decode_variable_byte_int},
    constants::SUBSCRIBE_PACKET_TYPE,
};

use super::{CommonPacketError, DecodablePacket, Packet};

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
#[allow(dead_code)]
pub(crate) struct TopicFilter {
    pub topic: String,
    qos_level: u8,
    no_local: bool,
    retain_as_published: bool,
    retain_handling: RetainHandling,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct SubscribePacket {
    packet_id: u16,
    pub topics: Vec<TopicFilter>,
}

#[derive(Debug)]
pub(crate) enum SubscribePacketDecodeError {
    Common(CommonPacketError),
}

impl std::error::Error for SubscribePacketDecodeError {}

impl fmt::Display for SubscribePacketDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Common(e) => write!(f, "Common Error: {e}"),
        }
    }
}

impl Packet for SubscribePacket {
    fn packet_type() -> u8 {
        SUBSCRIBE_PACKET_TYPE
    }
}

impl DecodablePacket for SubscribePacket {
    type Error = SubscribePacketDecodeError;

    fn validate_header(fixed_header: u8) -> Result<(), Self::Error> {
        let packet_type = fixed_header >> 4;
        if packet_type != Self::packet_type() {
            let e = CommonPacketError::MalformedPacket(Some(format!(
                "Invalid packet type: {packet_type}"
            )));
            return Err(Self::Error::Common(e));
        }

        // Reserved flags (4 LSB) must be equal to 0010
        let flags = fixed_header & 0b0000_1111;
        if flags != 0x2 {
            let e =
                CommonPacketError::MalformedPacket(Some("Fixed header flags are reserved".into()));
            return Err(Self::Error::Common(e));
        }

        Ok(())
    }

    fn decode(
        cursor: &mut Cursor<&[u8]>,
        _fixed_header: u8,
        _remaining_len: usize,
    ) -> Result<Self, Self::Error> {
        let packet_id = cursor.get_u16();

        let properties_len = decode_variable_byte_int(cursor).map_err(Self::Error::Common)?;
        cursor.advance(properties_len);

        let mut topics = Vec::new();
        while cursor.has_remaining() {
            let topic = decode_utf8_string(cursor).map_err(Self::Error::Common)?;

            let subscription_options = cursor.get_u8();

            //  It is a protocol error if the maximum QoS field has the value 3
            let qos_level = subscription_options & 0b0000_0011;
            if qos_level > 2 {
                let e = CommonPacketError::ProtocolError(Some(format!(
                    "Maximum QoS must be 0, 1 or 2. Got: {qos_level}"
                )));
                return Err(Self::Error::Common(e));
            }

            let no_local = subscription_options >> 3 & 1 == 1;

            let retain_as_published = subscription_options >> 4 & 1 == 1;

            let raw_retain_handling = subscription_options >> 5 & 0b0000_0011;
            let Some(retain_handling) = RetainHandling::from_u8(raw_retain_handling) else {
                // It is a Protocol Error to send a Retain Handling value of 3
                let e = CommonPacketError::ProtocolError(Some(format!(
                    "Retain handling must be 0, 1 or 2. Got: {raw_retain_handling}"
                )));
                return Err(Self::Error::Common(e));
            };

            let reserved_bits = subscription_options >> 7 & 0b0000_0011;
            if reserved_bits != 0x0 {
                // If any of reserved bits in the payload are non-zero, it must be treated as malformed packet
                let e = CommonPacketError::MalformedPacket(Some(
                    "Subscription options bits 6 and 7 are reserved".into(),
                ));
                return Err(Self::Error::Common(e));
            }
            topics.push(TopicFilter {
                topic,
                qos_level,
                no_local,
                retain_as_published,
                retain_handling,
            });
        }

        Ok(Self { packet_id, topics })
    }
}
