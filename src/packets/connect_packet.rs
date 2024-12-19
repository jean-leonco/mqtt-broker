use std::{collections::HashMap, io::Cursor};

use bytes::{Buf, BytesMut};

use crate::{
    codec::{decode_binary_data, decode_utf8_string, decode_variable_byte_int},
    connection::PacketError,
    constants::{PROTOCOL_NAME, PROTOCOL_VERSION},
};

/// Represents a decoded MQTT CONNECT packet as defined in the MQTT protocol.
#[derive(Debug)]
pub(crate) struct ConnectPacket {
    /// The Protocol Name is a UTF-8 Encoded String that represents the protocol name “MQTT”.
    pub protocol_name: String,

    /// Represents the revision level of the protocol used by the Client. The value of the Protocol Version field for version 5.0 of the protocol is 5 (0x05).
    pub protocol_version: u8,

    /// Specifies whether the Connection starts a new Session or is a continuation of an existing Session.
    pub clean_start: bool,

    /// Specifies the `QoS` level to be used when publishing the Will Message.
    qos_level: u8,

    /// Specifies if the Will Message is to be retained when it is published.
    will_retain: bool,

    /// Keep alive time interval measured in seconds.
    ///
    /// It is the maximum time interval that is permitted to elapse between the point at which the Client finishes transmitting one MQTT Control Packet and the point it starts sending the next.
    pub keep_alive: u16,

    /// Session Expiry Interval in seconds.
    ///
    /// If the Session Expiry Interval is 0xFFFFFFFF (`UINT_MAX`), the Session does not expire.
    pub session_expiry_interval: Option<u32>,

    /// The Client uses this value to limit the number of `QoS` 1 and `QoS` 2 publications that it is willing to process concurrently. There is no mechanism to limit the `QoS` 0 publications that the Server might try to send.
    ///
    /// The value of Receive Maximum applies only to the current Network Connection. If the Receive Maximum value is absent then its value defaults to 65,535.
    receive_maximum: Option<u16>,

    /// Represents the Maximum Packet Size the Client is willing to accept. If the Maximum Packet Size is not present, no limit on the packet size is imposed beyond the limitations in the protocol as a result of the remaining length encoding and the protocol header sizes.
    maximum_packet_size: Option<u32>,

    /// This value indicates the highest value that the Client will accept as a Topic Alias sent by the Server. The Client uses this value to limit the number of Topic Aliases that it is willing to hold on this Connection.
    ///
    /// A value of 0 indicates that the Client does not accept any Topic Aliases on this connection. If Topic Alias Maximum is absent or zero, the Server MUST NOT send any Topic Aliases to the Client.
    topic_alias_maximum: Option<u16>,

    /// The Client uses this value to request the Server to return Response Information in the CONNACK. A value of 0 indicates that the Server MUST NOT return Response Information. If the value is 1 the Server MAY return Response Information in the CONNACK packet.
    request_response_information: Option<bool>,

    /// The Client uses this value to indicate whether the Reason String or User Properties are sent in the case of failures.
    ///
    /// If the value of Request Problem Information is 0, the Server MAY return a Reason String or User Properties on a CONNACK or DISCONNECT packet, but MUST NOT send a Reason String or User Properties on any packet other than PUBLISH, CONNACK, or DISCONNECT.
    ///
    /// If this value is 1, the Server MAY return a Reason String or User Properties on any packet where it is allowed.
    request_problem_information: Option<bool>,

    /// User Properties on the CONNECT packet can be used to send connection related properties from the Client to the Server.
    user_properties: Option<HashMap<String, String>>,

    /// Contains the name of the authentication method used for extended authentication.
    authentication_method: Option<String>,

    /// The contents of this data are defined by the authentication method.
    authentication_data: Option<BytesMut>,

    /// The Client Identifier (`ClientID`) identifies the Client to the Server. Each Client connecting to the Server has a unique `ClientID`.
    pub client_id: String,

    /// The will topic.
    will_topic: Option<String>,

    /// It can be used by the Server for authentication and authorization.
    username: Option<String>,

    /// Although this field is called Password, it can be used to carry any credential information.
    password: Option<BytesMut>,
}

#[derive(Debug)]
pub(crate) enum DecodeError {
    PacketError(PacketError),
    UnsupportedProtocolVersion,
    ClientIdentifierNotValid,
}

impl ConnectPacket {
    /// Decode a input reader into the `ConnectPacket`.
    pub fn decode(buf: &mut Cursor<&[u8]>) -> Result<Self, DecodeError> {
        let protocol_name = decode_utf8_string(buf).map_err(DecodeError::PacketError)?;
        if protocol_name != PROTOCOL_NAME {
            return Err(DecodeError::UnsupportedProtocolVersion);
        }

        let protocol_version = buf.get_u8();
        if protocol_version != PROTOCOL_VERSION {
            return Err(DecodeError::UnsupportedProtocolVersion);
        }

        let connect_flags = buf.get_u8();

        // The Server MUST validate that the reserved flag in the CONNECT packet is set to 0
        if connect_flags & 1 != 0 {
            return Err(DecodeError::PacketError(PacketError::MalformedPacket));
        }

        let clean_start = (connect_flags >> 1) & 1 == 1;
        let will_flag = (connect_flags >> 2) & 1 == 1;
        let qos_level = (connect_flags >> 3) & 0b0000_0011;
        let will_retain = (connect_flags >> 5) & 1 == 1;
        let password_flag = (connect_flags >> 6) & 1 == 1;
        let username_flag = (connect_flags >> 7) & 1 == 1;

        // If the Will Flag is set to 0, then Will Retain MUST be set to 0
        if !will_flag && will_retain {
            return Err(DecodeError::PacketError(PacketError::MalformedPacket));
        }

        // If the Will Flag is set to 0, then the Will QoS MUST be set to 0
        if !will_flag && qos_level != 0 {
            return Err(DecodeError::PacketError(PacketError::MalformedPacket));
        }

        // If the Will Flag is set to 1, the value of Will QoS can be 0, 1, or 2. A value of 3 is a Malformed Packet
        if qos_level > 2 {
            return Err(DecodeError::PacketError(PacketError::MalformedPacket));
        }

        let keep_alive = buf.get_u16();

        let properties_len = decode_variable_byte_int(buf).map_err(DecodeError::PacketError)?;

        let (
            session_expiry_interval,
            receive_maximum,
            maximum_packet_size,
            topic_alias_maximum,
            request_response_information,
            request_problem_information,
            user_properties,
            authentication_method,
            authentication_data,
        ) = match properties_len {
            0 => (None, None, None, None, None, None, None, None, None),
            _ => {
                // TODO: Decode properties
                buf.advance(properties_len);
                (None, None, None, None, None, None, None, None, None)
            }
        };

        let client_id = decode_utf8_string(buf).map_err(DecodeError::PacketError)?;

        if client_id.is_empty()
            || client_id.len() > 23
            || !client_id.chars().all(char::is_alphanumeric)
        {
            return Err(DecodeError::ClientIdentifierNotValid);
        }

        // TODO: Add will_properties and will_payload

        let will_topic = if will_flag {
            let will_topic = decode_utf8_string(buf).map_err(DecodeError::PacketError)?;
            Some(will_topic)
        } else {
            None
        };

        let username = if username_flag {
            let username = decode_utf8_string(buf).map_err(DecodeError::PacketError)?;
            Some(username)
        } else {
            None
        };

        let password = if password_flag {
            let password = decode_binary_data(buf).map_err(DecodeError::PacketError)?;
            Some(password)
        } else {
            None
        };

        Ok(Self {
            protocol_name,
            protocol_version,
            clean_start,
            qos_level,
            will_retain,
            keep_alive,
            session_expiry_interval,
            receive_maximum,
            maximum_packet_size,
            topic_alias_maximum,
            request_response_information,
            request_problem_information,
            user_properties,
            authentication_method,
            authentication_data,
            client_id,
            will_topic,
            username,
            password,
        })
    }
}
