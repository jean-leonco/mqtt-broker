use std::{collections::HashMap, io::Read};

use anyhow::{Context, Ok};
use log::{debug, warn};

use crate::protocol;

// TODO: Add will_properties and will_payload
#[derive(Debug)]
pub(crate) struct ConnectPacket {
    /// The Protocol Name is a UTF-8 Encoded String that represents the protocol name “MQTT”.
    protocol_name: String,

    /// Represents the revision level of the protocol used by the Client. The value of the Protocol Version field for version 5.0 of the protocol is 5 (0x05).
    protocol_version: u8,

    /// Specifies whether the Connection starts a new Session or is a continuation of an existing Session.
    clean_start: bool,

    /// If the Will Flag is set to 1 this indicates that a Will Message MUST be stored on the Server and associated with the Session.
    will_flag: bool,

    /// Specifies the `QoS` level to be used when publishing the Will Message.
    qos_level: u8,

    /// Specifies if the Will Message is to be retained when it is published.
    will_retain: bool,

    ///  Specifies if the username is present in the payload.
    username_flag: bool,

    ///  Specifies if the password is present in the payload.
    password_flag: bool,

    ///  Keep alive time interval measured in seconds.
    ///
    /// It is the maximum time interval that is permitted to elapse between the point at which the Client finishes transmitting one MQTT Control Packet and the point it starts sending the next.
    keep_alive: u16,

    /// Session Expiry Interval in seconds.
    ///
    /// If it is set to 0, or is absent, the Session ends when the Network Connection is closed.
    ///
    /// If the Session Expiry Interval is 0xFFFFFFFF (`UINT_MAX`), the Session does not expire.
    session_expiry_interval: Option<u32>,

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
    authentication_data: Option<Vec<u8>>,

    /// The Client Identifier (`ClientID`) identifies the Client to the Server. Each Client connecting to the Server has a unique `ClientID`.
    client_id: String,

    /// The will topic.
    will_topic: Option<String>,

    /// It can be used by the Server for authentication and authorization.
    username: Option<String>,

    /// Although this field is called Password, it can be used to carry any credential information.
    password: Option<Vec<u8>>,
}

impl ConnectPacket {
    pub fn decode(stream: &mut impl Read) -> anyhow::Result<Self> {
        debug!("Decoding Connect packet");

        let remaining_len = protocol::decode_variable_byte_int(stream)
            .context("Failed to decode remaining length")?;
        debug!("Packet remaining_len: {}", remaining_len);

        // The Protocol Name is a UTF-8 Encoded String that represents the protocol name “MQTT”
        let protocol_name =
            protocol::decode_utf8_string(stream).context("Failed to decode protocol name")?;
        debug!("Packet protocol_name: {}", protocol_name);

        if protocol_name != protocol::PROTOCOL_NAME {
            warn!("Unsupported protocol name: {}", protocol_name);
            anyhow::bail!("Unsupported Protocol Version")
        }

        let mut buf = [0; 1];

        // The one byte unsigned value that represents the revision level of the protocol used by the Client.
        stream.read_exact(&mut buf).context("Failed to read protocol version")?;
        let protocol_version = u8::from_be_bytes(buf);
        debug!("Packet protocol_version: {}", protocol_version);

        if protocol_version != protocol::PROTOCOL_VERSION {
            warn!("Unsupported protocol version: {}", protocol_version);
            anyhow::bail!("Unsupported Protocol Version")
        }

        // The Connect Flags byte contains several parameters specifying the behavior of the MQTT connection. It also indicates the presence or absence of fields in the Payload
        stream.read_exact(&mut buf).context("Failed to read connect flags")?;
        let connect_flags = buf[0];
        debug!("Packet connect_flags: {:08b}", connect_flags);

        // The Server MUST validate that the reserved flag in the CONNECT packet is set to 0
        if connect_flags & 1 != 0 {
            warn!("Malformed packet: reserved flag set in packet");
            return Err(anyhow::anyhow!("Malformed packet"));
        }

        let clean_start = (connect_flags >> 1) & 1 == 1;
        let will_flag = (connect_flags >> 2) & 1 == 1;
        let qos_level = (connect_flags >> 3) & 0b0000_0011;
        let will_retain = (connect_flags >> 5) & 1 == 1;
        let password_flag = (connect_flags >> 6) & 1 == 1;
        let username_flag = (connect_flags >> 7) & 1 == 1;

        // If the Will Flag is set to 0, then Will Retain MUST be set to 0
        if !will_flag && will_retain {
            warn!("Malformed packet: will_flag is 0 but will_retain is set");
            return Err(anyhow::anyhow!("Malformed packet"));
        }

        // If the Will Flag is set to 0, then the Will QoS MUST be set to 0 (0x00)
        if !will_flag && qos_level != 0 {
            warn!(
                "Malformed packet: Invalid will_qos when will_flag is disabled or QoS out of range"
            );
            return Err(anyhow::anyhow!("Malformed packet"));
        }

        // If the Will Flag is set to 1, the value of Will QoS can be 0 (0x00), 1 (0x01), or 2 (0x02). A value of 3 (0x03) is a Malformed Packet
        if qos_level > 2 {
            warn!("Malformed packet: will_qos out of range");
            return Err(anyhow::anyhow!("Malformed packet"));
        }

        let mut keep_alive_buf = [0; 2];
        stream.read_exact(&mut keep_alive_buf).context("Failed to read keep alive")?;
        let keep_alive = u16::from_be_bytes(keep_alive_buf);
        debug!("Keep alive: {}", keep_alive);

        let properties_len = protocol::decode_variable_byte_int(stream)
            .context("Failed to decode properties len")?;
        debug!("Properties length: {}", properties_len);

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
            // TODO: Decode properties
            _ => (None, None, None, None, None, None, None, None, None),
        };

        // These fields, if present, MUST appear in the order Client Identifier, Will Properties, Will Topic, Will Payload, User Name, Password
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;
        debug!("Encoded: {}", hex::encode(buf));

        Ok(Self {
            protocol_name,
            protocol_version,
            clean_start,
            will_flag,
            qos_level,
            will_retain,
            username_flag,
            password_flag,
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
            client_id: String::from("client-001"),
            will_topic: None,
            username: None,
            password: None,
        })
    }
}
