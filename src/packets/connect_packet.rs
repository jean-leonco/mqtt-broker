use std::{collections::HashMap, fmt, io::Cursor};

use bytes::{Buf, Bytes, BytesMut};

use crate::{
    codec::{decode_binary_data, decode_utf8_string, decode_variable_byte_int},
    constants::{CONNECT_PACKET_TYPE, PROTOCOL_NAME, PROTOCOL_VERSION},
};

use super::{CommonPacketError, DecodablePacket};

/// Represents a decoded MQTT CONNECT packet as defined in the MQTT protocol.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ConnectPacket {
    /// The protocol name is a utf-8 encoded string that represents the protocol name “MQTT”.
    pub protocol_name: String,

    /// Represents the revision level of the protocol used by the client.
    pub protocol_version: u8,

    /// Specifies whether the connection starts a new session or is a continuation of an existing session.
    pub clean_start: bool,

    /// Specifies the `QoS` level to be used when publishing the will message.
    qos_level: u8,

    /// Specifies if the will message is to be retained when it is published.
    will_retain: bool,

    /// It is the maximum time interval that is permitted to elapse between the point at which the Client finishes transmitting one MQTT Control Packet and the point it starts sending the next.
    pub keep_alive: u16,

    /// Connect properties.
    pub properties: ConnectPacketProperties,

    /// The Client Identifier identifies the Client to the Server.
    pub client_id: String,

    /// The will topic.
    will_topic: Option<String>,

    /// It can be used by the Server for authentication and authorization.
    username: Option<String>,

    /// Although this field is called Password, it can be used to carry any credential information.
    password: Option<BytesMut>,
}

#[derive(Debug)]
pub(crate) enum ConnectPacketDecodeError {
    Common(CommonPacketError),
    UnsupportedProtocolVersion(Option<String>),
    ClientIdentifierNotValid(Option<String>),
}

impl std::error::Error for ConnectPacketDecodeError {}

impl fmt::Display for ConnectPacketDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Common(e) => write!(f, "Common Error: {e}"),

            Self::UnsupportedProtocolVersion(Some(reason)) => {
                write!(f, "Unsupported Protocol Version: {reason}")
            }
            Self::UnsupportedProtocolVersion(None) => write!(f, "Unsupported Protocol Version"),

            Self::ClientIdentifierNotValid(Some(reason)) => {
                write!(f, "Client Identifier Not Valid: {reason}")
            }
            Self::ClientIdentifierNotValid(None) => write!(f, "Client Identifier Not Valid"),
        }
    }
}

impl DecodablePacket for ConnectPacket {
    type Error = ConnectPacketDecodeError;

    fn packet_type() -> u8 {
        CONNECT_PACKET_TYPE
    }

    fn validate_header(fixed_header: u8) -> Result<(), Self::Error> {
        let packet_type = fixed_header >> 4;
        if packet_type != Self::packet_type() {
            let e = CommonPacketError::MalformedPacket(Some(format!(
                "Invalid packet type: {packet_type}"
            )));
            return Err(Self::Error::Common(e));
        }

        // Reserved flags (4 LSB) must be equal to 0000
        let flags = fixed_header << 4;
        if flags != 0x0 {
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
        let protocol_name = decode_utf8_string(cursor).map_err(Self::Error::Common)?;
        if protocol_name != PROTOCOL_NAME {
            let e = Some(format!("Unsupported protocol name: {protocol_name}"));
            return Err(Self::Error::UnsupportedProtocolVersion(e));
        }

        let protocol_version = cursor.get_u8();
        if protocol_version != PROTOCOL_VERSION {
            let e = Some(format!("Unsupported protocol version: {protocol_version}"));
            return Err(Self::Error::UnsupportedProtocolVersion(e));
        }

        let connect_flags = cursor.get_u8();

        // Reserved connect flag (last bit) must be set to 0
        if connect_flags & 0x1 != 0x0 {
            let e = CommonPacketError::MalformedPacket(Some("Connect flags are reserved".into()));
            return Err(Self::Error::Common(e));
        }

        let clean_start = connect_flags >> 1 & 1 == 1;
        let will_flag = connect_flags >> 2 & 1 == 1;
        let qos_level = connect_flags >> 3 & 0b0000_0011;
        let will_retain = connect_flags >> 5 & 1 == 1;
        let password_flag = connect_flags >> 6 & 1 == 1;
        let username_flag = connect_flags >> 7 & 1 == 1;

        // If the will_flag is set to 0, then will_retain must be set to 0
        if !will_flag && will_retain {
            let e = CommonPacketError::MalformedPacket(Some(
                "Will retain must be 0 if will flag is 1".into(),
            ));
            return Err(Self::Error::Common(e));
        }

        // If the will_flag is set to 0, then will_qos must be set to 0
        if !will_flag && qos_level != 0 {
            let e = CommonPacketError::MalformedPacket(Some(
                "Will QoS must be 0 if will flag is 0".into(),
            ));
            return Err(Self::Error::Common(e));
        }

        // If the will_flag is set to 1, then will_qos can be 0, 1, or 2
        if qos_level > 2 {
            let e = CommonPacketError::MalformedPacket(Some(format!(
                "Will QoS must be 0, 1 or 2. Got: {qos_level}"
            )));
            return Err(Self::Error::Common(e));
        }

        let keep_alive = cursor.get_u16();

        let properties_len = decode_variable_byte_int(cursor).map_err(Self::Error::Common)?;

        let properties = ConnectPacketProperties::decode(cursor, properties_len)?;

        let client_id = decode_utf8_string(cursor).map_err(Self::Error::Common)?;

        if client_id.is_empty()
            || client_id.len() > 23
            || !client_id.chars().all(char::is_alphanumeric)
        {
            let e = Some(format!("Client identifier is not valid"));
            return Err(Self::Error::ClientIdentifierNotValid(e));
        }

        // TODO: Add will_properties and will_payload

        let will_topic = if will_flag {
            let will_topic = decode_utf8_string(cursor).map_err(Self::Error::Common)?;
            Some(will_topic)
        } else {
            None
        };

        let username = if username_flag {
            let username = decode_utf8_string(cursor).map_err(Self::Error::Common)?;
            Some(username)
        } else {
            None
        };

        let password = if password_flag {
            let password = decode_binary_data(cursor).map_err(Self::Error::Common)?;
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
            properties,
            client_id,
            will_topic,
            username,
            password,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ConnectPacketProperties {
    /// Session Expiry Interval in seconds.
    /// If the Session Expiry Interval is 0xFFFFFFFF (`UINT_MAX`), the Session does not expire.
    pub session_expiry_interval: Option<u32>,

    /// The Client uses this value to limit the number of `QoS` 1 and `QoS` 2 publications that it is willing to process concurrently.
    /// If the Receive Maximum value is absent then its value defaults to 65,535.
    receive_maximum: Option<u16>,

    /// Represents the Maximum Packet Size the Client is willing to accept. If the Maximum Packet Size is not present, no limit on the packet size is imposed beyond the limitations in the protocol.
    maximum_packet_size: Option<u32>,

    /// This value indicates the highest value that the Client will accept as a Topic Alias sent by the Server.
    /// If Topic Alias Maximum is absent, the Server MUST NOT send any Topic Aliases to the Client.
    topic_alias_maximum: Option<u16>,

    /// The Client uses this value to request the Server to return Response Information in the CONNACK.
    request_response_information: Option<bool>,

    /// The Client uses this value to indicate whether the Reason String or User Properties are sent in the case of failures.
    request_problem_information: Option<bool>,

    /// User Properties on the CONNECT packet can be used to send connection related properties from the Client to the Server.
    user_properties: Option<HashMap<String, String>>,

    /// Contains the name of the authentication method used for extended authentication.
    authentication_method: Option<String>,

    /// The data used to authenticate.
    authentication_data: Option<Bytes>,
}

impl ConnectPacketProperties {
    fn decode(
        cursor: &mut Cursor<&[u8]>,
        len: usize,
    ) -> Result<ConnectPacketProperties, ConnectPacketDecodeError> {
        if len > 0 {
            cursor.advance(len);
        }

        Ok(ConnectPacketProperties {
            session_expiry_interval: None,
            receive_maximum: None,
            maximum_packet_size: None,
            topic_alias_maximum: None,
            request_response_information: None,
            request_problem_information: None,
            user_properties: None,
            authentication_method: None,
            authentication_data: None,
        })
    }
}
