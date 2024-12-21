use std::collections::HashMap;
use std::fmt;

use bytes::{BufMut, Bytes, BytesMut};

use crate::codec::{
    encode_variable_byte_int, write_usize_as_var_int, write_utf8_string, write_utf8_string_pair,
};
use crate::constants::{CONNACK_PACKET_TYPE, MAX_PACKET_SIZE};

use super::{CommonPacketError, EncodablePacket};

pub const SESSION_EXPIRY_INTERVAL_IDENTIFIER: u8 = 0x11;
pub const RECEIVE_MAXIMUM_IDENTIFIER: u8 = 0x21;
pub const MAXIMUM_QOS_IDENTIFIER: u8 = 0x24;
pub const RETAIN_AVAILABLE_IDENTIFIER: u8 = 0x25;
pub const MAXIMUM_PACKET_SIZE_IDENTIFIER: u8 = 0x27;
pub const ASSIGNED_CLIENT_IDENTIFIER: u8 = 0x12;
pub const TOPIC_ALIAS_MAXIMUM_IDENTIFIER: u8 = 0x22;
pub const REASON_STRING_IDENTIFIER: u8 = 0x1F;
pub const USER_PROPERTY_IDENTIFIER: u8 = 0x26;
pub const WILDCARD_SUBSCRIPTION_AVAILABLE_IDENTIFIER: u8 = 0x28;
pub const SUBSCRIPTION_IDENTIFIERS_AVAILABLE_IDENTIFIER: u8 = 0x29;
pub const SHARED_SUBSCRIPTION_AVAILABLE_IDENTIFIER: u8 = 0x2A;
pub const SERVER_KEEP_ALIVE_IDENTIFIER: u8 = 0x13;
pub const RESPONSE_INFORMATION_IDENTIFIER: u8 = 0x1A;
pub const SERVER_REFERENCE_IDENTIFIER: u8 = 0x1C;
pub const AUTHENTICATION_METHOD_IDENTIFIER: u8 = 0x15;
pub const AUTHENTICATION_DATA_IDENTIFIER: u8 = 0x16;

/// Represents the possible reason codes returned when attempting to connect to an MQTT server.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum ConnectReasonCode {
    /// Success connection is accepted.
    Success = 0x00,

    /// The server does not wish to reveal the reason for the failure, or none of the other reason codes apply.
    UnspecifiedError = 0x80,

    /// Data within the connect packet could not be correctly parsed.
    MalformedPacket = 0x81,

    /// Data in the connect packet does not conform to this specification.
    ProtocolError = 0x82,

    /// The connect is valid but is not accepted by this server.
    ImplementationSpecificError = 0x83,

    /// The server does not support the version of the mqtt protocol requested by the client.
    UnsupportedProtocolVersion = 0x84,

    /// The client identifier is a valid string but is not allowed by the server.
    ClientIdentifierNotValid = 0x85,

    /// The server does not accept the user name or password specified by the client.
    BadUserNameOrPassword = 0x86,

    /// The client is not authorized to connect.
    NotAuthorized = 0x87,

    /// The mqtt server is not available.
    ServerUnavailable = 0x88,

    /// The server is busy. try again later.
    ServerBusy = 0x89,

    /// This client has been banned by administrative action. contact the server administrator.
    Banned = 0x8A,

    /// The authentication method is not supported or does not match the authentication method currently in use.
    BadAuthenticationMethod = 0x8C,

    /// The will topic name is not malformed, but is not accepted by this server.
    TopicNameInvalid = 0x90,

    /// The connect packet exceeded the maximum permissible size.
    PacketTooLarge = 0x95,

    /// An implementation or administrative imposed limit has been exceeded.
    QuotaExceeded = 0x97,

    /// The will payload does not match the specified payload format indicator.
    PayloadFormatInvalid = 0x99,

    /// The server does not support retained messages, and will retain was set to 1.
    RetainNotSupported = 0x9A,

    /// The server does not support the `qos` set in will `qos`.
    QosNotSupported = 0x9B,

    /// The client should temporarily use another server.
    UseAnotherServer = 0x9C,

    /// The client should permanently use another server.
    ServerMoved = 0x9D,

    /// The connection rate limit has been exceeded.
    ConnectionRateExceeded = 0x9F,
}

impl ConnectReasonCode {
    fn to_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for ConnectReasonCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Success => "Success",
            Self::UnspecifiedError => "Unspecified error",
            Self::MalformedPacket => "Malformed packet",
            Self::ProtocolError => "Protocol error",
            Self::ImplementationSpecificError => "Implementation specific error",
            Self::UnsupportedProtocolVersion => "Unsupported protocol version",
            Self::ClientIdentifierNotValid => "Client identifier not valid",
            Self::BadUserNameOrPassword => "Bad user name or password",
            Self::NotAuthorized => "Not authorized",
            Self::ServerUnavailable => "Server unavailable",
            Self::ServerBusy => "Server busy",
            Self::Banned => "Banned",
            Self::BadAuthenticationMethod => "Bad authentication method",
            Self::TopicNameInvalid => "Topic name invalid",
            Self::PacketTooLarge => "Packet too large",
            Self::QuotaExceeded => "Quota exceeded",
            Self::PayloadFormatInvalid => "Payload format invalid",
            Self::RetainNotSupported => "Retain not supported",
            Self::QosNotSupported => "QoS not supported",
            Self::UseAnotherServer => "Use another server",
            Self::ServerMoved => "Server moved",
            Self::ConnectionRateExceeded => "Connection rate exceeded",
        };

        write!(f, "{value}")
    }
}

/// The CONNACK packet is the packet sent by the Server in response to a CONNECT packet received from a Client.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ConnAckPacket {
    /// It informs the client whether the server is using session state from a previous connection for this client id.
    /// If a server sends a connack packet containing a non-zero reason code it must set session present to 0.
    session_present: bool,

    /// If a well formed CONNECT packet is received by the server, but the server is unable to complete the connection the server may send a CONNACK packet containing the appropriate Connect Reason code.
    reason_code: ConnectReasonCode,

    /// Connection acknowledgment properties.
    properties: ConnAckPacketProperties,
}

#[derive(Debug)]
pub(crate) enum ConnAckPacketEncodeError {
    Common(CommonPacketError),
}

impl std::error::Error for ConnAckPacketEncodeError {}

impl fmt::Display for ConnAckPacketEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Common(e) => write!(f, "Common Error: {e}"),
        }
    }
}

impl EncodablePacket for ConnAckPacket {
    type Error = ConnAckPacketEncodeError;

    fn encode(&self) -> Result<bytes::BytesMut, Self::Error> {
        let mut properties_buf = BytesMut::new();
        self.properties.write_properties(&mut properties_buf)?;

        let properties_len = encode_variable_byte_int(
            u32::try_from(properties_buf.len())
                .map_err(|_| Self::Error::Common(CommonPacketError::IntOverflow))?,
        );

        let mut buf = BytesMut::new();

        // Fixed header
        buf.put_u8(CONNACK_PACKET_TYPE << 4);

        // Remaining length: acknowledge flags + reason code + properties len + properties
        let remaining_len = 2 + properties_len.len() + properties_buf.len();
        write_usize_as_var_int(&mut buf, remaining_len).map_err(Self::Error::Common)?;

        // Reason code
        buf.put_u8(self.reason_code.to_u8());

        // Acknowledge flags
        buf.put_u8(self.session_present.into());

        // Properties
        buf.put(&properties_len[..]);
        buf.put(properties_buf);

        if buf.len() > MAX_PACKET_SIZE {
            return Err(Self::Error::Common(CommonPacketError::PacketTooLarge(None)));
        }

        Ok(buf)
    }
}

impl ConnAckPacket {
    pub(crate) fn builder() -> ConnAckPacketBuilder<NeedsSessionPresent> {
        ConnAckPacketBuilder(NeedsSessionPresent(()))
    }
}

pub struct ConnAckPacketBuilder<State>(State);

pub struct NeedsSessionPresent(());

pub struct NeedsReasonCode {
    session_present: bool,
}

pub struct ReadyToBuild {
    session_present: bool,
    reason_code: ConnectReasonCode,
    reason_string: Option<String>,
}

impl ConnAckPacketBuilder<NeedsSessionPresent> {
    pub(crate) fn session_present(
        self,
        session_present: bool,
    ) -> ConnAckPacketBuilder<NeedsReasonCode> {
        ConnAckPacketBuilder(NeedsReasonCode { session_present })
    }
}

impl ConnAckPacketBuilder<NeedsReasonCode> {
    pub(crate) fn reason_code(
        self,
        reason_code: ConnectReasonCode,
    ) -> ConnAckPacketBuilder<ReadyToBuild> {
        ConnAckPacketBuilder(ReadyToBuild {
            session_present: self.0.session_present,
            reason_code,
            reason_string: None,
        })
    }
}

impl ConnAckPacketBuilder<ReadyToBuild> {
    //pub(crate) fn reason_string(self, reason_string: String) -> ConnAckPacketBuilder<ReadyToBuild> {
    //    ConnAckPacketBuilder(ReadyToBuild { reason_string: Some(reason_string), ..self.0 })
    //}

    pub(crate) fn build(self) -> ConnAckPacket {
        ConnAckPacket {
            session_present: self.0.session_present,
            reason_code: self.0.reason_code,
            properties: ConnAckPacketProperties {
                session_expiry_interval: None,
                receive_maximum: None,
                maximum_qos: None,
                retain_available: None,
                maximum_packet_size: None,
                assigned_client_identifier: None,
                topic_alias_maximum: Some(u16::MAX),
                reason_string: self.0.reason_string,
                user_properties: None,
                wildcard_subscription_available: None,
                subscription_identifiers_available: None,
                shared_subscription_available: None,
                server_keep_alive: None,
                response_information: None,
                server_reference: None,
                authentication_method: None,
                authentication_data: None,
            },
        }
    }
}

#[derive(Debug)]
struct ConnAckPacketProperties {
    /// Session expiry interval in seconds.
    session_expiry_interval: Option<u32>,

    /// The Server uses this value to limit the number of `QoS` 1 and `QoS` 2 publications that it is willing to process concurrently for the Client. It does not provide a mechanism to limit the `QoS` 0 publications that the Client might try to send.
    receive_maximum: Option<u16>,

    /// If a Server does not support `QoS` 1 or `QoS` 2 PUBLISH packets it MUST send a Maximum `QoS` in the CONNACK packet specifying the highest `QoS` it supports.
    maximum_qos: Option<u8>,

    /// It declares whether the Server supports retained messages. If not present, then retained messages are supported.
    retain_available: Option<bool>,

    /// The Maximum Packet Size the Server is willing to accept. If the Maximum Packet Size is not present, there is no limit on the packet size imposed beyond the limitations in the protocol.
    maximum_packet_size: Option<u32>,

    /// The Client Identifier which was assigned by the Server because a zero length Client Identifier was found in the CONNECT packet.
    assigned_client_identifier: Option<String>,

    /// This value indicates the highest value that the Server will accept as a Topic Alias sent by the Client. The Server uses this value to limit the number of Topic Aliases that it is willing to hold on this Connection.
    topic_alias_maximum: Option<u16>,

    /// Human readable string designed for diagnostics and SHOULD NOT be parsed by the Client.
    reason_string: Option<String>,

    /// User Properties that can be used to provide additional information to the Client including diagnostic information.
    user_properties: Option<HashMap<String, String>>,

    /// Declares whether the Server supports Wildcard Subscriptions. If not present, then Wildcard Subscriptions are supported.
    wildcard_subscription_available: Option<bool>,

    /// Declares whether the Server supports Subscription Identifiers. If not present, then Subscription Identifiers are supported.
    subscription_identifiers_available: Option<bool>,

    /// Declares whether the Server supports Shared Subscriptions. If not present, then Shared Subscriptions are supported.
    shared_subscription_available: Option<bool>,

    /// If the Server sends a Server Keep Alive on the CONNACK packet, the Client MUST use this value instead of the Keep Alive value the Client sent on CONNECT. If the Server does not send the Server Keep Alive, the Server MUST use the Keep Alive value set by the Client on CONNECT.
    server_keep_alive: Option<u16>,

    /// Basis for creating a Response Topic.
    response_information: Option<String>,

    /// It can be used by the Client to identify another Server to use.
    ///
    /// The Server uses a Server Reference in either a CONNACK or DISCONNECT packet with Reason code of 0x9C (Use another server) or Reason Code 0x9D (Server moved).
    server_reference: Option<String>,

    /// It contains the name of the authentication method.
    authentication_method: Option<String>,

    /// The contents of this data are defined by the authentication method.
    authentication_data: Option<Bytes>,
}

impl ConnAckPacketProperties {
    fn write_properties(&self, buf: &mut BytesMut) -> Result<(), ConnAckPacketEncodeError> {
        if let Some(session_expiry_interval) = self.session_expiry_interval {
            buf.put_u8(SESSION_EXPIRY_INTERVAL_IDENTIFIER);
            buf.put_u32(session_expiry_interval);
        }

        if let Some(receive_maximum) = self.receive_maximum {
            buf.put_u8(RECEIVE_MAXIMUM_IDENTIFIER);
            buf.put_u16(receive_maximum);
        }

        if let Some(maximum_qos) = self.maximum_qos {
            buf.put_u8(MAXIMUM_QOS_IDENTIFIER);
            buf.put_u8(maximum_qos);
        }

        if let Some(retain_available) = self.retain_available {
            buf.put_u8(RETAIN_AVAILABLE_IDENTIFIER);
            buf.put_u8(retain_available.into());
        }

        if let Some(maximum_packet_size) = self.maximum_packet_size {
            buf.put_u8(MAXIMUM_PACKET_SIZE_IDENTIFIER);
            buf.put_u32(maximum_packet_size);
        }

        if let Some(assigned_client_identifier) = &self.assigned_client_identifier {
            buf.put_u8(ASSIGNED_CLIENT_IDENTIFIER);
            write_utf8_string(buf, assigned_client_identifier)
                .map_err(ConnAckPacketEncodeError::Common)?;
        }

        if let Some(topic_alias_maximum) = self.topic_alias_maximum {
            buf.put_u8(TOPIC_ALIAS_MAXIMUM_IDENTIFIER);
            buf.put_u16(topic_alias_maximum);
        }

        if let Some(reason_string) = &self.reason_string {
            buf.put_u8(REASON_STRING_IDENTIFIER);
            write_utf8_string(buf, reason_string).map_err(ConnAckPacketEncodeError::Common)?;
        }

        if let Some(user_properties) = &self.user_properties {
            for user_property in user_properties {
                buf.put_u8(USER_PROPERTY_IDENTIFIER);
                write_utf8_string_pair(buf, user_property)
                    .map_err(ConnAckPacketEncodeError::Common)?;
            }
        }

        if let Some(wildcard_subscription_available) = self.wildcard_subscription_available {
            buf.put_u8(WILDCARD_SUBSCRIPTION_AVAILABLE_IDENTIFIER);
            buf.put_u8(wildcard_subscription_available.into());
        }

        if let Some(subscription_identifiers_available) = self.subscription_identifiers_available {
            buf.put_u8(SUBSCRIPTION_IDENTIFIERS_AVAILABLE_IDENTIFIER);
            buf.put_u8(subscription_identifiers_available.into());
        }

        if let Some(shared_subscription_available) = self.shared_subscription_available {
            buf.put_u8(SHARED_SUBSCRIPTION_AVAILABLE_IDENTIFIER);
            buf.put_u8(shared_subscription_available.into());
        }

        if let Some(server_keep_alive) = self.server_keep_alive {
            buf.put_u8(SERVER_KEEP_ALIVE_IDENTIFIER);
            buf.put_u16(server_keep_alive);
        }

        if let Some(response_information) = &self.response_information {
            buf.put_u8(RESPONSE_INFORMATION_IDENTIFIER);
            write_utf8_string(buf, response_information)
                .map_err(ConnAckPacketEncodeError::Common)?;
        }

        if let Some(server_reference) = &self.server_reference {
            buf.put_u8(SERVER_REFERENCE_IDENTIFIER);
            write_utf8_string(buf, server_reference).map_err(ConnAckPacketEncodeError::Common)?;
        }

        if let Some(authentication_method) = &self.authentication_method {
            buf.put_u8(AUTHENTICATION_METHOD_IDENTIFIER);
            write_utf8_string(buf, authentication_method)
                .map_err(ConnAckPacketEncodeError::Common)?;
        }

        if let Some(authentication_data) = &self.authentication_data {
            buf.put_u8(AUTHENTICATION_DATA_IDENTIFIER);
            buf.put(&authentication_data[..]);
        }

        Ok(())
    }
}
