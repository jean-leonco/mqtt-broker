use std::{error::Error, fmt, io::Cursor};

use bytes::BytesMut;

pub(crate) mod conn_ack_packet;
pub(crate) mod connect_packet;
pub(crate) mod disconnect_packet;
pub(crate) mod ping_req_packet;
pub(crate) mod ping_resp_packet;
pub(crate) mod publish_packet;
pub(crate) mod subscribe_packet;

pub(crate) trait DecodablePacket: Sized {
    type Error: Error + 'static + Send + Sync;

    fn packet_type() -> u8;

    fn validate_header(fixed_header: u8) -> Result<(), Self::Error>;

    fn decode(
        cursor: &mut Cursor<&[u8]>,
        fixed_header: u8,
        remaining_len: usize,
    ) -> Result<Self, Self::Error>;
}

pub(crate) trait EncodablePacket: Sized {
    type Error: Error + 'static + Send + Sync;

    fn encode(&self) -> Result<BytesMut, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum CommonPacketError {
    PacketTooLarge(Option<String>),
    MalformedPacket(Option<String>),
    ProtocolError(Option<String>),
    UnexpectedError,
    IntOverflow,
}

impl fmt::Display for CommonPacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PacketTooLarge(Some(reason)) => write!(f, "Packet Too Large: {reason}"),
            Self::PacketTooLarge(None) => write!(f, "Packet Too Large"),

            Self::MalformedPacket(Some(reason)) => write!(f, "Malformed Packet: {reason}"),
            Self::MalformedPacket(None) => write!(f, "Malformed Packet"),

            Self::ProtocolError(Some(reason)) => write!(f, "Protocol Error: {reason}"),
            Self::ProtocolError(None) => write!(f, "Protocol Error"),

            Self::UnexpectedError => write!(f, "Unexpected Error"),

            Self::IntOverflow => write!(f, "Variable Int Overflow"),
        }
    }
}
