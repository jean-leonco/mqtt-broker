use anyhow::Context;
use log::debug;
use std::{collections::HashMap, fmt};

use crate::protocol::{encoding, packet_type::PacketType, MAX_PACKET_SIZE};

const SESSION_EXPIRY_INTERVAL_IDENTIFIER: u8 = 0x11;
const REASON_STRING_IDENTIFIER: u8 = 0x1F;
const USER_PROPERTY_IDENTIFIER: u8 = 0x26;
const SERVER_REFERENCE_IDENTIFIER: u8 = 0x1C;

/// Represents the Reason Codes sent when disconnecting from an MQTT connection.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum DisconnectReasonCode {
    /// Close the connection normally. Do not send the Will Message.
    /// Sent by: Client or Server.
    NormalDisconnection = 0x00,

    /// The Client wishes to disconnect but requires that the Server also publishes its Will Message.
    /// Sent by: Client.
    DisconnectWithWillMessage = 0x04,

    /// The Connection is closed but the sender either does not wish to reveal the reason, or none of the other Reason Codes apply.
    /// Sent by: Client or Server.
    UnspecifiedError = 0x80,

    /// The received packet does not conform to this specification.
    /// Sent by: Client or Server.
    MalformedPacket = 0x81,

    /// An unexpected or out of order packet was received.
    /// Sent by: Client or Server.
    ProtocolError = 0x82,

    /// The packet received is valid but cannot be processed by this implementation.
    /// Sent by: Client or Server.
    ImplementationSpecificError = 0x83,

    /// The request is not authorized.
    /// Sent by: Server.
    NotAuthorized = 0x87,

    /// The Server is busy and cannot continue processing requests from this Client.
    /// Sent by: Server.
    ServerBusy = 0x89,

    /// The Server is shutting down.
    /// Sent by: Server.
    ServerShuttingDown = 0x8B,

    /// The Connection is closed because no packet has been received for 1.5 times the Keep alive time.
    /// Sent by: Server.
    KeepAliveTimeout = 0x8D,

    /// Another Connection using the same `ClientID` has connected causing this Connection to be closed.
    /// Sent by: Server.
    SessionTakenOver = 0x8E,

    /// The Topic Filter is correctly formed, but is not accepted by this Server.
    /// Sent by: Server.
    TopicFilterInvalid = 0x8F,

    /// The Topic Name is correctly formed, but is not accepted by this Client or Server.
    /// Sent by: Client or Server.
    TopicNameInvalid = 0x90,

    /// The Client or Server has received more than Receive Maximum publications for which it has not sent PUBACK or PUBCOMP.
    /// Sent by: Client or Server.
    ReceiveMaximumExceeded = 0x93,

    /// The Client or Server has received a PUBLISH packet containing a Topic Alias which is greater than the Maximum Topic Alias it sent in the CONNECT or CONNACK packet.
    /// Sent by: Client or Server.
    TopicAliasInvalid = 0x94,

    /// The packet size is greater than Maximum Packet Size for this Client or Server.
    /// Sent by: Client or Server.
    PacketTooLarge = 0x95,

    /// The received data rate is too high.
    /// Sent by: Client or Server.
    MessageRateTooHigh = 0x96,

    /// An implementation or administrative imposed limit has been exceeded.
    /// Sent by: Client or Server.
    QuotaExceeded = 0x97,

    /// The Connection is closed due to an administrative action.
    /// Sent by: Client or Server.
    AdministrativeAction = 0x98,

    /// The payload format does not match the one specified by the Payload Format Indicator.
    /// Sent by: Client or Server.
    PayloadFormatInvalid = 0x99,

    /// The Server does not support retained messages.
    /// Sent by: Server.
    RetainNotSupported = 0x9A,

    /// The Client specified a `QoS` greater than the `QoS` specified in a Maximum `QoS` in the CONNACK.
    /// Sent by: Server.
    QosNotSupported = 0x9B,

    /// The Client should temporarily change its Server.
    /// Sent by: Server.
    UseAnotherServer = 0x9C,

    /// The Server has moved and the Client should permanently change its server location.
    /// Sent by: Server.
    ServerMoved = 0x9D,

    /// The Server does not support Shared Subscriptions.
    /// Sent by: Server.
    SharedSubscriptionsNotSupported = 0x9E,

    /// This connection is closed because the connection rate is too high.
    /// Sent by: Server.
    ConnectionRateExceeded = 0x9F,

    /// The maximum connection time authorized for this connection has been exceeded.
    /// Sent by: Server.
    MaximumConnectTime = 0xA0,

    /// The Server does not support Subscription Identifiers; the subscription is not accepted.
    /// Sent by: Server.
    SubscriptionIdentifiersNotSupported = 0xA1,

    /// The Server does not support Wildcard Subscriptions; the subscription is not accepted.
    /// Sent by: Server.
    WildcardSubscriptionsNotSupported = 0xA2,
}

impl DisconnectReasonCode {
    /// Converts the `DisconnectReasonCode` to its numeric value.
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for DisconnectReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::NormalDisconnection => "Normal disconnection",
            Self::DisconnectWithWillMessage => "Disconnect with will message",
            Self::UnspecifiedError => "Unspecified error",
            Self::MalformedPacket => "Malformed packet",
            Self::ProtocolError => "Protocol error",
            Self::ImplementationSpecificError => "Implementation specific error",
            Self::NotAuthorized => "Not authorized",
            Self::ServerBusy => "Server busy",
            Self::ServerShuttingDown => "Server shutting down",
            Self::KeepAliveTimeout => "Keep alive timeout",
            Self::SessionTakenOver => "Session taken over",
            Self::TopicFilterInvalid => "Topic filter invalid",
            Self::TopicNameInvalid => "Topic name invalid",
            Self::ReceiveMaximumExceeded => "Receive maximum exceeded",
            Self::TopicAliasInvalid => "Topic alias invalid",
            Self::PacketTooLarge => "Packet too large",
            Self::MessageRateTooHigh => "Message rate too high",
            Self::QuotaExceeded => "Quota exceeded",
            Self::AdministrativeAction => "Administrative action",
            Self::PayloadFormatInvalid => "Payload format invalid",
            Self::RetainNotSupported => "Retain not supported",
            Self::QosNotSupported => "QoS not supported",
            Self::UseAnotherServer => "Use another server",
            Self::ServerMoved => "Server moved",
            Self::SharedSubscriptionsNotSupported => "Shared subscriptions not supported",
            Self::ConnectionRateExceeded => "Connection rate exceeded",
            Self::MaximumConnectTime => "Maximum connect time",
            Self::SubscriptionIdentifiersNotSupported => "Subscription identifiers not supported",
            Self::WildcardSubscriptionsNotSupported => "Wildcard subscriptions not supported",
        };
        write!(f, "{value}")
    }
}

/// The DISCONNECT packet is the final MQTT Control Packet sent from the Client or the Server.
/// It indicates the reason why the Network Connection is being closed.
/// The Client or Server MAY send a DISCONNECT packet before closing the Network Connection.
/// If the Connection closes without sending a DISCONNECT packet with Reason Code 0x00 (Normal disconnection)
/// and a Will Message is in place, the Will Message is published.
#[derive(Debug)]
pub(crate) struct DisconnectPacket {
    /// The Reason Code indicating why the DISCONNECT is occurring.
    reason_code: DisconnectReasonCode,

    /// Represents the Session Expiry Interval in seconds.
    /// The Session Expiry Interval MUST NOT be sent on a DISCONNECT by the Server.
    session_expiry_interval: Option<u32>,

    /// A human-readable reason string for diagnostic purposes. Should NOT be parsed programmatically.
    reason_string: Option<String>,

    /// User properties in key-value pairs.
    user_properties: Option<HashMap<String, String>>, // TODO: The same name is allowed to appear more than once

    /// Encoded String which can be used by the Client to identify another Server to use.
    server_reference: Option<String>,

    remaining_len: usize,
}

impl DisconnectPacket {
    /// Creates a `DisconnectPacket` with the provided parameters, while ensuring that the values comply with the MQTT protocol requirements.
    ///
    /// # Errors
    /// - Returns an error if:
    ///   - `server_reference` is not a valid encoded UTF-8 string..
    ///   - `server_reference` is present, but `reason_code` is not `UseAnotherServer` or `ServerMoved`.
    ///   - `reason_string` is not a valid encoded UTF-8 string.
    pub fn new(
        reason_code: DisconnectReasonCode,
        session_expiry_interval: Option<u32>,
        reason_string: Option<String>,
        user_properties: Option<HashMap<String, String>>,
        server_reference: Option<String>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            reason_code,
            session_expiry_interval,
            reason_string,
            user_properties,
            server_reference,
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
        let encoded_remaining_len = encoding::encode_variable_byte_int(remaining_len);

        let control_byte = PacketType::Disconnect.control_byte();

        // Allocate enough space for the control packet type, flags and remaining length
        let mut header = Vec::with_capacity(1 + encoded_remaining_len.len());

        // Append the control packet, flags and remaining length to fixed header
        header.push(control_byte);
        header.extend(encoded_remaining_len);

        Ok(header)
    }

    /// Encode the `DisconnectPacket` into bytes.
    /// This attempts to respect MQTT property size constraints and will omit properties that don't fit.
    ///
    /// # Errors
    /// Returns an error if packet size exceeds `MAX_PACKET_SIZE`.
    pub fn encode(&mut self) -> anyhow::Result<Vec<u8>> {
        // Check if we have any properties at all
        let has_properties = self.session_expiry_interval.is_some()
            || self.reason_string.is_some()
            || self.user_properties.is_some()
            || self.server_reference.is_some();

        // If reason code is NormalDisconnection (0x00) and no properties, we can omit Reason Code and Property Length.
        if matches!(self.reason_code, DisconnectReasonCode::NormalDisconnection) && !has_properties
        {
            debug!("NormalDisconnection with no properties: sending minimal packet");
            return self.encode_fixed_header().context("Failed to encode packet fixed header");
        }

        let mut properties = Vec::new();

        // Add Session Expiry Interval if present
        if let Some(session_expiry_interval) = self.session_expiry_interval {
            properties.push(SESSION_EXPIRY_INTERVAL_IDENTIFIER);
            properties.extend(session_expiry_interval.to_be_bytes());
        }

        // Add Server Reference if present
        if let Some(server_reference) = &self.server_reference {
            // TODO: The sender MUST NOT send this Property if it would increase the size of the DISCONNECT packet beyond the Maximum Packet Size specified by the receiver.

            let server_reference = encoding::encode_utf8_string(server_reference)
                .context("Failed to encode server_reference")?;

            properties.push(SERVER_REFERENCE_IDENTIFIER);
            properties.extend(server_reference);
        }

        // Add Reason String if present
        if let Some(reason_string) = &self.reason_string {
            // TODO: The sender MUST NOT send this Property if it would increase the size of the DISCONNECT packet beyond the Maximum Packet Size specified by the receiver.

            let reason_string = encoding::encode_utf8_string(reason_string)
                .context("Failed to encode reason_string")?;

            properties.push(REASON_STRING_IDENTIFIER);
            properties.extend(reason_string);
        }

        // Add User Properties if present
        if let Some(user_properties) = &self.user_properties {
            // TODO: The sender MUST NOT send this Property if it would increase the size of the DISCONNECT packet beyond the Maximum Packet Size specified by the receiver.

            for user_property in user_properties {
                properties.push(USER_PROPERTY_IDENTIFIER);
                properties.extend(
                    encoding::encode_utf8_string_pair(user_property)
                        .context("Failed to encode user_property")?,
                );
            }
        }

        let properties_len = u32::try_from(properties.len()).with_context(|| {
            format!("Failed to cast properties_len {} to u32", properties.len())
        })?;
        let properties_len = encoding::encode_variable_byte_int(properties_len);

        // Remaining Length: Reason code + Property length + Properties
        self.remaining_len += 1 + properties_len.len() + properties.len();

        let fixed_header =
            self.encode_fixed_header().context("Failed to encode packet fixed header")?;

        // Build packet: Fixed header + Reason code + Property length + Properties
        let mut packet = Vec::with_capacity(fixed_header.len() + self.remaining_len);
        packet.extend(fixed_header);
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
