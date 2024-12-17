use std::collections::HashMap;
use std::fmt;

use anyhow::Context;
use log::debug;

use crate::protocol::{
    encoding::{encode_utf8_string, encode_utf8_string_pair, encode_variable_byte_int},
    packet_type::PacketType,
    validation::validate_utf8_string,
    MAX_PACKET_SIZE,
};

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

/// Represents the possible reason codes returned when attempting
/// to connect to an MQTT server. Each variant corresponds to a specific
/// connection outcome or error.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ConnectReasonCode {
    /// Success Connection is accepted.
    Success = 0x00,

    /// The Server does not wish to reveal the reason for the failure,
    /// or none of the other Reason Codes apply.
    UnspecifiedError = 0x80,

    /// Data within the CONNECT packet could not be correctly parsed.
    MalformedPacket = 0x81,

    /// Data in the CONNECT packet does not conform to this specification.
    ProtocolError = 0x82,

    /// The CONNECT is valid but is not accepted by this Server.
    ImplementationSpecificError = 0x83,

    /// The Server does not support the version of the MQTT protocol requested by the Client.
    UnsupportedProtocolVersion = 0x84,

    /// The Client Identifier is a valid string but is not allowed by the Server.
    ClientIdentifierNotValid = 0x85,

    /// The Server does not accept the User Name or Password specified by the Client.
    BadUserNameOrPassword = 0x86,

    /// The Client is not authorized to connect.
    NotAuthorized = 0x87,

    /// The MQTT Server is not available.
    ServerUnavailable = 0x88,

    /// The Server is busy. Try again later.
    ServerBusy = 0x89,

    /// This Client has been banned by administrative action. Contact the server administrator.
    Banned = 0x8A,

    /// The authentication method is not supported or does not match the authentication method currently in use.
    BadAuthenticationMethod = 0x8C,

    /// The Will Topic Name is not malformed, but is not accepted by this Server.
    TopicNameInvalid = 0x90,

    /// The CONNECT packet exceeded the maximum permissible size.
    PacketTooLarge = 0x95,

    /// An implementation or administrative imposed limit has been exceeded.
    QuotaExceeded = 0x97,

    /// The Will Payload does not match the specified Payload Format Indicator.
    PayloadFormatInvalid = 0x99,

    /// The Server does not support retained messages, and Will Retain was set to 1.
    RetainNotSupported = 0x9A,

    /// The Server does not support the `QoS` set in Will `QoS`.
    QosNotSupported = 0x9B,

    /// The Client should temporarily use another server.
    UseAnotherServer = 0x9C,

    /// The Client should permanently use another server.
    ServerMoved = 0x9D,

    /// The connection rate limit has been exceeded.
    ConnectionRateExceeded = 0x9F,
}

impl ConnectReasonCode {
    /// Converts the `ConnectReasonCode` to its numeric value.
    pub fn to_u8(self) -> u8 {
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

/// Maximum supported `QoS`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MaximumQos {
    Zero = 0,
    One = 1,
}

impl MaximumQos {
    /// Converts the `MaximumQos` to its numeric value.
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// The CONNACK packet is the packet sent by the Server in response to a CONNECT packet received from a Client.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ConnAckPacket {
    /// The Session Present flag informs the Client whether the Server is using Session State from a previous connection for this `ClientID`.
    ///
    /// If a Server sends a CONNACK packet containing a non-zero Reason Code it MUST set Session Present to 0.
    session_present: bool,

    /// If a well formed CONNECT packet is received by the Server, but the Server is unable to complete the Connection the Server MAY send a CONNACK packet containing the appropriate Connect Reason code. If a Server sends a CONNACK packet containing a Reason code of 128 or greater it MUST then close the Network Connection.
    reason_code: ConnectReasonCode,

    /// Session Expiry Interval in seconds.
    session_expiry_interval: Option<u32>,

    /// The Server uses this value to limit the number of QoS 1 and QoS 2 publications that it is willing to process concurrently for the Client. It does not provide a mechanism to limit the QoS 0 publications that the Client might try to send.
    receive_maximum: Option<u16>,

    /// If a Server does not support QoS 1 or QoS 2 PUBLISH packets it MUST send a Maximum QoS in the CONNACK packet specifying the highest QoS it supports.
    maximum_qos: Option<MaximumQos>,

    /// If present, this byte declares whether the Server supports retained messages. A value of 0 means that retained messages are not supported. A value of 1 means retained messages are supported. If not present, then retained messages are supported.
    retain_available: Option<bool>,

    /// The Maximum Packet Size the Server is willing to accept. If the Maximum Packet Size is not present, there is no limit on the packet size imposed beyond the limitations in the protocol as a result of the remaining length encoding and the protocol header sizes.
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
    authentication_data: Option<Vec<u8>>,

    remaining_len: usize,
}

impl ConnAckPacket {
    pub fn new(
        session_present: bool,
        reason_code: ConnectReasonCode,
        session_expiry_interval: Option<u32>,
        receive_maximum: Option<u16>,
        maximum_qos: Option<MaximumQos>,
        retain_available: Option<bool>,
        maximum_packet_size: Option<u32>,
        assigned_client_identifier: Option<String>,
        topic_alias_maximum: Option<u16>,
        reason_string: Option<String>,
        user_properties: Option<HashMap<String, String>>,
        wildcard_subscription_available: Option<bool>,
        subscription_identifiers_available: Option<bool>,
        shared_subscription_available: Option<bool>,
        server_keep_alive: Option<u16>,
        response_information: Option<String>,
        server_reference: Option<String>,
        authentication_method: Option<String>,
        authentication_data: Option<Vec<u8>>,
    ) -> anyhow::Result<Self> {
        if let Some(ref assigned_client_identifier) = assigned_client_identifier {
            validate_utf8_string(assigned_client_identifier)
                .context("assigned_client_identifier is not a valid utf8 string")?;
        }

        if let Some(ref reason_string) = reason_string {
            validate_utf8_string(reason_string)
                .context("reason_string is not a valid utf8 string")?;
        }

        if let Some(ref response_information) = response_information {
            validate_utf8_string(response_information)
                .context("response_information is not a valid utf8 string")?;
        }

        if let Some(ref server_reference) = server_reference {
            validate_utf8_string(server_reference)
                .context("server_reference is not a valid utf8 string")?;

            if !(matches!(
                reason_code,
                ConnectReasonCode::UseAnotherServer | ConnectReasonCode::ServerMoved
            )) {
                anyhow::bail!(
                    "Expected reason_code to be {} or {}. Got {reason_code}",
                    ConnectReasonCode::UseAnotherServer,
                    ConnectReasonCode::ServerMoved
                );
            }
        }

        if let Some(ref authentication_method) = authentication_method {
            validate_utf8_string(authentication_method)
                .context("authentication_method is not a valid utf8 string")?;
        }

        Ok(Self {
            session_present,
            reason_code,
            session_expiry_interval,
            receive_maximum,
            maximum_qos,
            retain_available,
            maximum_packet_size,
            assigned_client_identifier,
            topic_alias_maximum,
            reason_string,
            user_properties,
            wildcard_subscription_available,
            subscription_identifiers_available,
            shared_subscription_available,
            server_keep_alive,
            response_information,
            server_reference,
            authentication_method,
            authentication_data,
            remaining_len: 0,
        })
    }

    /// Encode the fixed header for the control packet.
    ///
    /// The MQTT fixed header is composed of:
    /// - The control byte (the packet type and flags)
    /// - The remaining length (variable-length integer)
    ///
    /// # Fixed Header Format
    ///
    /// | Bit       | 7   | 6   | 5   | 4   | 3   | 2   | 1   | 0   |
    /// |-----------|-----|-----|-----|-----|-----|-----|-----|-----|
    /// | Byte 1    | Packet type           | Packet flags          |
    /// | Byte 2    | Remaining Length                              |
    fn encode_fixed_header(&self) -> anyhow::Result<Vec<u8>> {
        let remaining_len = u32::try_from(self.remaining_len).with_context(|| {
            format!("Failed to cast remaining length {} to u32", self.remaining_len)
        })?;
        let encoded_remaining_len = encode_variable_byte_int(remaining_len);

        let control_byte = PacketType::ConnAck.control_byte();

        // Allocate enough space for the control packet type, flags and remaining length
        let mut header = Vec::with_capacity(1 + encoded_remaining_len.len());

        // Append the control packet, flags and remaining length to fixed header
        header.push(control_byte);
        header.extend(encoded_remaining_len);

        Ok(header)
    }

    pub fn encode(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut properties = Vec::new();

        // Add Session Expiry Interval if present
        if let Some(session_expiry_interval) = self.session_expiry_interval {
            properties.push(SESSION_EXPIRY_INTERVAL_IDENTIFIER);
            properties.extend(session_expiry_interval.to_be_bytes());
        }

        // Add Receive Maximum if present
        if let Some(receive_maximum) = self.receive_maximum {
            properties.push(RECEIVE_MAXIMUM_IDENTIFIER);
            properties.extend(receive_maximum.to_be_bytes());
        }

        // Add Maximum QoS if present
        if let Some(maximum_qos) = self.maximum_qos {
            properties.push(MAXIMUM_QOS_IDENTIFIER);
            properties.push(maximum_qos.to_u8());
        }

        // Add Retain available if present
        if let Some(retain_available) = self.retain_available {
            let retain_available = u8::from(retain_available);

            properties.push(RETAIN_AVAILABLE_IDENTIFIER);
            properties.push(retain_available);
        }

        // Add Maximum Packet Size if present
        if let Some(maximum_packet_size) = self.maximum_packet_size {
            properties.push(MAXIMUM_PACKET_SIZE_IDENTIFIER);
            properties.extend(maximum_packet_size.to_be_bytes());
        }

        // Add Assigned Client Identifier if present
        if let Some(assigned_client_identifier) = &self.assigned_client_identifier {
            properties.push(ASSIGNED_CLIENT_IDENTIFIER);
            properties.extend(
                encode_utf8_string(assigned_client_identifier)
                    .context("Failed to encode assigned_client_identifier")?,
            );
        }

        // Add Topic Alias Maximum if present
        if let Some(topic_alias_maximum) = &self.topic_alias_maximum {
            properties.push(TOPIC_ALIAS_MAXIMUM_IDENTIFIER);
            properties.extend(topic_alias_maximum.to_be_bytes());
        }

        // Add Reason String if present
        if let Some(reason_string) = &self.reason_string {
            properties.push(REASON_STRING_IDENTIFIER);
            properties.extend(
                encode_utf8_string(reason_string).context("Failed to encode reason_string")?,
            );
        }

        // Add Reason String if present
        if let Some(reason_string) = &self.reason_string {
            properties.push(REASON_STRING_IDENTIFIER);
            properties.extend(
                encode_utf8_string(reason_string).context("Failed to encode reason_string")?,
            );
        }

        // Add User Properties if present
        if let Some(user_properties) = &self.user_properties {
            for user_property in user_properties {
                properties.push(USER_PROPERTY_IDENTIFIER);
                properties.extend(
                    encode_utf8_string_pair(user_property)
                        .context("Failed to encode user_property")?,
                );
            }
        }

        // Add Wildcard Subscription Available if present
        if let Some(wildcard_subscription_available) = self.wildcard_subscription_available {
            let wildcard_subscription_available = u8::from(wildcard_subscription_available);

            properties.push(WILDCARD_SUBSCRIPTION_AVAILABLE_IDENTIFIER);
            properties.push(wildcard_subscription_available);
        }

        // Add Subscription Identifiers Available if present
        if let Some(subscription_identifiers_available) = self.subscription_identifiers_available {
            let subscription_identifiers_available = u8::from(subscription_identifiers_available);

            properties.push(SUBSCRIPTION_IDENTIFIERS_AVAILABLE_IDENTIFIER);
            properties.push(subscription_identifiers_available);
        }

        // Add Shared Subscription Available if present
        if let Some(shared_subscription_available) = self.shared_subscription_available {
            let shared_subscription_available = u8::from(shared_subscription_available);

            properties.push(SHARED_SUBSCRIPTION_AVAILABLE_IDENTIFIER);
            properties.push(shared_subscription_available);
        }

        // Add Server Keep Alive if present
        if let Some(server_keep_alive) = self.server_keep_alive {
            properties.push(SERVER_KEEP_ALIVE_IDENTIFIER);
            properties.extend(server_keep_alive.to_be_bytes());
        }

        // Add Response Information if present
        if let Some(response_information) = &self.response_information {
            properties.push(RESPONSE_INFORMATION_IDENTIFIER);
            properties.extend(
                encode_utf8_string(response_information)
                    .context("Failed to encode response_information")?,
            );
        }

        // Add Server Reference if present
        if let Some(server_reference) = &self.server_reference {
            properties.push(SERVER_REFERENCE_IDENTIFIER);
            properties.extend(
                encode_utf8_string(server_reference)
                    .context("Failed to encode server_reference")?,
            );
        }

        // Add Authentication Method if present
        if let Some(authentication_method) = &self.authentication_method {
            properties.push(AUTHENTICATION_METHOD_IDENTIFIER);
            properties.extend(
                encode_utf8_string(authentication_method)
                    .context("Failed to encode authentication_method")?,
            );
        }

        // Add Authentication Data if present
        if let Some(authentication_data) = &self.authentication_data {
            properties.push(AUTHENTICATION_DATA_IDENTIFIER);
            properties.extend(authentication_data);
        }

        let properties_len = u32::try_from(properties.len()).with_context(|| {
            format!("Failed to cast properties_len {} to u32", properties.len())
        })?;
        let properties_len = encode_variable_byte_int(properties_len);

        // Remaining Length: Acknowledge Flags + Reason Code + Property length + Properties
        self.remaining_len += 1 + 1 + properties_len.len() + properties.len();
        debug!("remaining_len: {}", self.remaining_len);

        let fixed_header =
            self.encode_fixed_header().context("Failed to encode packet fixed header")?;

        // Byte 1 is the "Connect Acknowledge Flags". Bits 7-1 are reserved and MUST be set to 0
        // Bit 0 is the Session Present Flag.
        let acknowledge_flags = u8::from(self.session_present);

        // Build packet: Fixed header + Acknowledge Flags + Reason Code + Property length + Properties
        let mut packet = Vec::with_capacity(fixed_header.len() + self.remaining_len);
        packet.extend(fixed_header);
        packet.push(acknowledge_flags);
        packet.push(self.reason_code.to_u8());
        packet.extend(properties_len);
        packet.extend(properties);

        // Ensure the packet isn't larger than the MAX_PACKET_SIZE
        if packet.len() > MAX_PACKET_SIZE {
            anyhow::bail!(
            "Packet size exceeds the maximum allowed value. Packet size: {}, Maximum allowed: {}",
            packet.len(), MAX_PACKET_SIZE
        );
        }

        Ok(packet)
    }
}
