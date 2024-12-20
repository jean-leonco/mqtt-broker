use std::{error::Error, fmt, io::Cursor};

use bytes::{Buf, BytesMut};
use log::error;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    codec::decode_variable_byte_int,
    constants::{
        AUTH_IDENTIFIER, CONNACK_IDENTIFIER, CONNECT_IDENTIFIER, DISCONNECT_IDENTIFIER,
        PINGREQ_IDENTIFIER, PINGRESP_IDENTIFIER, PUBACK_IDENTIFIER, PUBCOMP_IDENTIFIER,
        PUBLISH_IDENTIFIER, PUBREC_IDENTIFIER, PUBREL_IDENTIFIER, SUBACK_IDENTIFIER,
        SUBSCRIBE_IDENTIFIER, UNSUBACK_IDENTIFIER, UNSUBSCRIBE_IDENTIFIER,
    },
    packets::{
        conn_ack_packet::ConnAckPacket, connect_packet::ConnectPacket,
        disconnect_packet::DisconnectPacket, subscribe_packet::SubscribePacket,
    },
};

#[derive(Debug)]
pub(crate) enum IncomingPacket {
    Connect(ConnectPacket),
    Disconnect(DisconnectPacket),
    Subscribe(SubscribePacket),
    Publish,
    PingReq,
}

#[derive(Debug)]
pub(crate) enum OutgoingPacket {
    ConnAck(ConnAckPacket),
    Disconnect(DisconnectPacket),
    PingResp,
}

#[derive(Debug)]
pub(crate) enum PacketError {
    ConnectionReset,
    PacketTooLarge,
    MalformedPacket,
    UnexpectedError,
    ProtocolError,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::ConnectionReset => "Connection Reset",
            Self::PacketTooLarge => "Packet Too Large",
            Self::MalformedPacket => "Malformed Packet",
            Self::UnexpectedError => "Unexpected Error",
            Self::ProtocolError => "Protocol Error",
        };
        write!(f, "{value}")
    }
}

impl Error for PacketError {}

#[derive(Debug)]
pub(crate) struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection { stream, buffer: BytesMut::with_capacity(4096) }
    }

    /// Read a packet from the connection.
    ///
    /// Returns `None` if EOF is reached
    pub async fn read_packet(&mut self) -> Result<Option<IncomingPacket>, PacketError> {
        loop {
            if let Some(packet) = self.parse_packet()? {
                return Ok(Some(packet));
            }

            match self.stream.read_buf(&mut self.buffer).await {
                Ok(n) => {
                    if n == 0 {
                        if self.buffer.is_empty() {
                            return Ok(None);
                        }

                        return Err(PacketError::ConnectionReset);
                    }
                }
                Err(e) => {
                    error!("Error: {e}");
                    return Err(PacketError::UnexpectedError);
                }
            }
        }
    }

    fn parse_packet(&mut self) -> Result<Option<IncomingPacket>, PacketError> {
        if self.buffer.is_empty() || self.buffer.len() < 2 {
            return Ok(None);
        }

        let mut buf = Cursor::new(&self.buffer[..]);

        let fixed_header = buf.get_u8();
        let packet_type = fixed_header >> 4;

        match packet_type {
            CONNECT_IDENTIFIER => {
                let remaining_len = decode_variable_byte_int(&mut buf)?;
                if remaining_len > buf.remaining() {
                    return Ok(None);
                }

                let start = buf.position() as usize;

                let packet = ConnectPacket::decode(&mut buf)
                    .map_err(|e| {
                        error!("{:?}", e);
                    })
                    .unwrap();

                self.buffer.advance(start + remaining_len);

                Ok(Some(IncomingPacket::Connect(packet)))
            }
            DISCONNECT_IDENTIFIER => {
                let remaining_len = decode_variable_byte_int(&mut buf)?;
                if remaining_len > buf.remaining() {
                    return Ok(None);
                }

                let start = buf.position() as usize;

                let packet = DisconnectPacket::decode(&mut buf)
                    .map_err(|e| {
                        error!("{:?}", e);
                    })
                    .unwrap();

                self.buffer.advance(start + remaining_len);

                Ok(Some(IncomingPacket::Disconnect(packet)))
            }
            SUBSCRIBE_IDENTIFIER => {
                let remaining_len = decode_variable_byte_int(&mut buf)?;
                if remaining_len > buf.remaining() {
                    return Ok(None);
                }

                let start = buf.position() as usize;

                let packet = SubscribePacket::decode(&mut buf)
                    .map_err(|e| {
                        error!("{:?}", e);
                    })
                    .unwrap();

                self.buffer.advance(start + remaining_len);

                Ok(Some(IncomingPacket::Subscribe(packet)))
            }
            PUBLISH_IDENTIFIER => {
                let remaining_len = decode_variable_byte_int(&mut buf)?;
                if remaining_len > buf.remaining() {
                    return Ok(None);
                }

                let start = buf.position() as usize;
                self.buffer.advance(start + remaining_len);

                Ok(Some(IncomingPacket::Publish))
            }
            PINGREQ_IDENTIFIER => Ok(Some(IncomingPacket::PingReq)),
            PUBACK_IDENTIFIER
            | PUBREC_IDENTIFIER
            | PUBREL_IDENTIFIER
            | PUBCOMP_IDENTIFIER
            | UNSUBSCRIBE_IDENTIFIER
            | AUTH_IDENTIFIER => {
                todo!("Not implemeted: {packet_type}")
            }
            // Server to client
            CONNACK_IDENTIFIER | SUBACK_IDENTIFIER | UNSUBACK_IDENTIFIER | PINGRESP_IDENTIFIER => {
                Err(PacketError::ProtocolError)
            }
            _ => Err(PacketError::MalformedPacket),
        }
    }

    /// Write a packet to the connection.
    pub async fn write_packet(&mut self, packet: OutgoingPacket) -> io::Result<()> {
        // needs a trait that defines methods: encode_payload, encode_variable_header, encode_flags
        //
        // let payload = packet.encode_payload();
        // let variable_header = packet.encode_variable_header();
        // let remaining_len = encode_variable_byte_int(payload.len() + variable_header.len());
        //
        // let flags = packet.encode_flags();
        // let packet_id = match packet {};
        // let fixed_header = packet_id & flags;
        //
        // let packet = Bytes::with_capacity(payload.len() + variable_header.len() + remaining_len.len() + 1);
        //
        // packet.push(fixed_header);
        // packet.extend(remaining_len);
        //
        // packet.extend(variable_header);
        // packet.extend(payload);
        //
        let encoded_packet = match packet {
            OutgoingPacket::ConnAck(mut packet) => packet.encode().unwrap(),
            OutgoingPacket::Disconnect(mut packet) => packet.encode().unwrap(),
            OutgoingPacket::PingResp => vec![PINGRESP_IDENTIFIER << 4, 0],
        };

        self.stream.write_all(&encoded_packet).await?;
        self.stream.flush().await?;

        Ok(())
    }
}
