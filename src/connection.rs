use std::{error::Error, io::Cursor};

use bytes::{Buf, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    codec::decode_variable_byte_int,
    packets::{CommonPacketError, DecodablePacket, EncodablePacket},
};

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum ConnectionError {
    Common(CommonPacketError),
    Decode(Box<dyn Error + Send + Sync>),
    ConnectionReset,
}

#[derive(Debug)]
pub(crate) struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub(crate) fn new(stream: TcpStream) -> Connection {
        Connection { stream, buffer: BytesMut::with_capacity(4096) }
    }

    /// Read a packet from the connection.
    ///
    /// Returns `None` if EOF is reached
    pub(crate) async fn read_packet<P: DecodablePacket>(
        &mut self,
    ) -> Result<Option<P>, ConnectionError> {
        loop {
            if let Some(packet) = self.decode_packet::<P>()? {
                return Ok(Some(packet));
            }

            match self.stream.read_buf(&mut self.buffer).await {
                Ok(n) => {
                    if n == 0 {
                        if self.buffer.is_empty() {
                            return Ok(None);
                        }

                        return Err(ConnectionError::ConnectionReset);
                    }
                }
                Err(_) => {
                    return Err(ConnectionError::Common(CommonPacketError::UnexpectedError));
                }
            }
        }
    }

    pub(crate) async fn peek_packet_type(&mut self) -> Result<Option<u8>, ConnectionError> {
        loop {
            if let Some(fixed_header) = self.peek_fixed_header() {
                return Ok(Some(fixed_header >> 4));
            }

            match self.stream.read_buf(&mut self.buffer).await {
                Ok(n) => {
                    if n == 0 {
                        if self.buffer.is_empty() {
                            return Ok(None);
                        }

                        return Err(ConnectionError::ConnectionReset);
                    }
                }
                Err(_) => {
                    return Err(ConnectionError::Common(CommonPacketError::UnexpectedError));
                }
            }
        }
    }

    fn peek_fixed_header(&self) -> Option<u8> {
        if self.buffer.is_empty() || self.buffer.len() < 1 {
            return None;
        }

        return Some(self.buffer.chunk()[0]);
    }

    fn decode_packet<P: DecodablePacket>(&mut self) -> Result<Option<P>, ConnectionError> {
        if self.buffer.is_empty() || self.buffer.len() < 2 {
            return Ok(None);
        }

        let mut cursor = Cursor::new(&self.buffer[..]);

        let fixed_header = cursor.get_u8();
        P::validate_header(fixed_header).map_err(|e| ConnectionError::Decode(Box::new(e)))?;

        let remaining_len =
            decode_variable_byte_int(&mut cursor).map_err(ConnectionError::Common)?;
        if remaining_len > cursor.remaining() {
            return Ok(None);
        }

        let start = cursor.position() as usize;

        match P::decode(&mut cursor, fixed_header, remaining_len) {
            Ok(packet) => {
                self.buffer.advance(start + remaining_len);
                Ok(Some(packet))
            }
            Err(e) => Err(ConnectionError::Decode(Box::new(e))),
        }
    }

    /// Write a packet to the connection.
    pub(crate) async fn write_packet<P: EncodablePacket>(
        &mut self,
        packet: &P,
    ) -> Result<(), ConnectionError> {
        let encoded = packet.encode().map_err(|e| ConnectionError::Decode(Box::new(e)))?;
        self.stream
            .write_all(&encoded)
            .await
            .map_err(|_| ConnectionError::Common(CommonPacketError::UnexpectedError))?;
        self.stream
            .flush()
            .await
            .map_err(|_| ConnectionError::Common(CommonPacketError::UnexpectedError))?;

        Ok(())
    }
}
