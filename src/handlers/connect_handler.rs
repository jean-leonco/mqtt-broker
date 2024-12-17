use anyhow::Context;
use log::{debug, error};
use std::io::{Read, Write};

use crate::{
    packets::{
        conn_ack_packet::{ConnAckPacket, ConnectReasonCode},
        connect_packet::ConnectPacket,
    },
    protocol::{MalformedPacketError, PROTOCOL_NAME, PROTOCOL_VERSION},
};

pub fn handle_connect<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> anyhow::Result<()> {
    let mut response = handle_packet(ConnectPacket::decode(reader));
    debug!("Response: {response:?}");

    writer.write_all(&response.encode()?).context("Failed to write CONNACK packet")
}

fn handle_packet(packet: anyhow::Result<ConnectPacket>) -> ConnAckPacket {
    match packet {
        Ok(packet) => {
            if packet.protocol_name != PROTOCOL_NAME {
                return ConnAckPacket::builder()
                    .session_present(false)
                    .reason_code(ConnectReasonCode::UnsupportedProtocolVersion)
                    .reason_string(format!("Unsupported protocol name: {}", packet.protocol_name))
                    .build();
            }

            if packet.protocol_version != PROTOCOL_VERSION {
                return ConnAckPacket::builder()
                    .session_present(false)
                    .reason_code(ConnectReasonCode::UnsupportedProtocolVersion)
                    .reason_string(format!(
                        "Unsupported protocol version: {}",
                        packet.protocol_version
                    ))
                    .build();
            }

            if packet.client_id.is_empty() {
                return ConnAckPacket::builder()
                    .session_present(false)
                    .reason_code(ConnectReasonCode::ClientIdentifierNotValid)
                    .reason_string("Client Identifier is empty".to_string())
                    .build();
            }

            if packet.client_id.len() > 23 {
                return ConnAckPacket::builder()
                    .session_present(false)
                    .reason_code(ConnectReasonCode::ClientIdentifierNotValid)
                    .reason_string(
                        "Client Identifier too large. Expected value between 1 and 23 bytes"
                            .to_string(),
                    )
                    .build();
            }

            if !packet.client_id.chars().all(char::is_alphanumeric) {
                return ConnAckPacket::builder()
                    .session_present(false)
                    .reason_code(ConnectReasonCode::ClientIdentifierNotValid)
                    .reason_string(
                        "Client Identifier contains non-alphanumeric characters".to_string(),
                    )
                    .build();
            }

            ConnAckPacket::builder()
                .session_present(false)
                .reason_code(ConnectReasonCode::Success)
                .build()
        }
        Err(e) => {
            error!("Decoding error: {e}");

            if let Some(MalformedPacketError(reason)) = e.downcast_ref() {
                return ConnAckPacket::builder()
                    .session_present(false)
                    .reason_code(ConnectReasonCode::MalformedPacket)
                    .reason_string(reason.clone())
                    .build();
            }

            return ConnAckPacket::builder()
                .session_present(false)
                .reason_code(ConnectReasonCode::UnspecifiedError)
                .build();
        }
    }
}
