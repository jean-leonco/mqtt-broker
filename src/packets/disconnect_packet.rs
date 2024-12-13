use anyhow::Context;
use log::{debug, error, trace, warn}; // Add logging
use std::{collections::HashMap, fmt};

use crate::protocol;

const SESSION_EXPIRY_INTERVAL_IDENTIFIER: u8 = 0x11;
const REASON_STRING_IDENTIFIER: u8 = 0x1F;
const USER_PROPERTY_IDENTIFIER: u8 = 0x26;
const SERVER_REFERENCE_IDENTIFIER: u8 = 0x1C;

/// Represents the Reason Codes sent when disconnecting from an MQTT connection.
#[derive(Debug)]
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
    QoSNotSupported = 0x9B,

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

impl fmt::Display for DisconnectReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            DisconnectReasonCode::NormalDisconnection => "NormalDisconnection",
            DisconnectReasonCode::DisconnectWithWillMessage => "DisconnectWithWillMessage",
            DisconnectReasonCode::UnspecifiedError => "UnspecifiedError",
            DisconnectReasonCode::MalformedPacket => "MalformedPacket",
            DisconnectReasonCode::ProtocolError => "ProtocolError",
            DisconnectReasonCode::ImplementationSpecificError => "ImplementationSpecificError",
            DisconnectReasonCode::NotAuthorized => "NotAuthorized",
            DisconnectReasonCode::ServerBusy => "ServerBusy",
            DisconnectReasonCode::ServerShuttingDown => "ServerShuttingDown",
            DisconnectReasonCode::KeepAliveTimeout => "KeepAliveTimeout",
            DisconnectReasonCode::SessionTakenOver => "SessionTakenOver",
            DisconnectReasonCode::TopicFilterInvalid => "TopicFilterInvalid",
            DisconnectReasonCode::TopicNameInvalid => "TopicNameInvalid",
            DisconnectReasonCode::ReceiveMaximumExceeded => "ReceiveMaximumExceeded",
            DisconnectReasonCode::TopicAliasInvalid => "TopicAliasInvalid",
            DisconnectReasonCode::PacketTooLarge => "PacketTooLarge",
            DisconnectReasonCode::MessageRateTooHigh => "MessageRateTooHigh",
            DisconnectReasonCode::QuotaExceeded => "QuotaExceeded",
            DisconnectReasonCode::AdministrativeAction => "AdministrativeAction",
            DisconnectReasonCode::PayloadFormatInvalid => "PayloadFormatInvalid",
            DisconnectReasonCode::RetainNotSupported => "RetainNotSupported",
            DisconnectReasonCode::QoSNotSupported => "QoSNotSupported",
            DisconnectReasonCode::UseAnotherServer => "UseAnotherServer",
            DisconnectReasonCode::ServerMoved => "ServerMoved",
            DisconnectReasonCode::SharedSubscriptionsNotSupported => {
                "SharedSubscriptionsNotSupported"
            }
            DisconnectReasonCode::ConnectionRateExceeded => "ConnectionRateExceeded",
            DisconnectReasonCode::MaximumConnectTime => "MaximumConnectTime",
            DisconnectReasonCode::SubscriptionIdentifiersNotSupported => {
                "SubscriptionIdentifiersNotSupported"
            }
            DisconnectReasonCode::WildcardSubscriptionsNotSupported => {
                "WildcardSubscriptionsNotSupported"
            }
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
}

impl DisconnectPacket {
    /// Create a new `DisconnectPacket`.
    /// If `server_reference` is present, the `reason_code` must be `UseAnotherServer` or `ServerMoved`.
    /// Otherwise, it is considered a logic error.
    pub fn new(
        reason_code: DisconnectReasonCode,
        session_expiry_interval: Option<u32>,
        reason_string: Option<String>,
        user_properties: Option<HashMap<String, String>>,
        server_reference: Option<String>,
    ) -> Self {
        if let Some(ref server_reference) = server_reference {
            if !(matches!(
                reason_code,
                DisconnectReasonCode::UseAnotherServer | DisconnectReasonCode::ServerMoved
            )) {
                warn!("Server reference '{server_reference}' provided but reason_code is {reason_code}, expected UseAnotherServer or ServerMoved.");
                unreachable!(
                    "Expected reason_code to be {} or {}. Got {reason_code}",
                    DisconnectReasonCode::UseAnotherServer,
                    DisconnectReasonCode::ServerMoved
                );
            }
        }

        Self {
            reason_code,
            session_expiry_interval,
            reason_string,
            user_properties,
            server_reference,
        }
    }

    /// Create the fixed header for the DISCONNECT packet.
    fn create_fixed_header(&self, remaining_length: u32) -> Vec<u8> {
        // BIT       7  6  5  4     3 2 1 0
        // BYTE 1:   DISCONNECT     RESERVED
        // BYTE 2..:   REMAINING_LENGTH
        match remaining_length {
            0 => {
                let mut header = vec![0; 2];
                header[0] = protocol::DISCONNECT << 4; // shift code to left (most significant bits)
                header[1] = 0x0;

                header
            }
            remaining_length => {
                let encoded_remaining_length = protocol::encode_variable_byte_int(remaining_length);
                let mut header = vec![0; 1 + encoded_remaining_length.len()];

                header[0] = protocol::DISCONNECT << 4; // shift code to left (most significant bits)
                header.extend(encoded_remaining_length);

                header
            }
        }
    }

    /// Serialize the `DisconnectPacket` into bytes.
    /// This attempts to respect MQTT property size constraints and will omit properties that don't fit.
    ///
    /// # Errors
    /// Returns an error if remaining length exceeds `MAX_ALLOWED_LENGTH`.
    pub fn serialize(self) -> anyhow::Result<Vec<u8>> {
        trace!("Serializing DisconnectPacket with reason_code = {}", self.reason_code);

        // Check if we have any properties at all
        let has_properties = self.session_expiry_interval.is_some()
            || self.reason_string.is_some()
            || self.user_properties.is_some()
            || self.server_reference.is_some();

        // If reason code is NormalDisconnection (0x00) and no properties, we can omit Reason Code and Property Length.
        if matches!(self.reason_code, DisconnectReasonCode::NormalDisconnection) && !has_properties
        {
            debug!("NormalDisconnection with no properties: sending minimal packet");
            // remaining_length = 0
            return Ok(self.create_fixed_header(0));
        }

        // Precompute total size of variable header properties
        let mut properties = Vec::new();
        let mut remaining_length = 0;

        // Add Session Expiry Interval if present
        if let Some(session_expiry_interval) = self.session_expiry_interval {
            properties.push(SESSION_EXPIRY_INTERVAL_IDENTIFIER);
            properties.extend(session_expiry_interval.to_be_bytes());

            remaining_length += 5;
        }

        // Add Server Reference if present
        if let Some(server_reference) = &self.server_reference {
            let bytes = server_reference.as_bytes();
            let len = 1 + bytes.len();

            if remaining_length + len <= protocol::MAX_ALLOWED_LENGTH {
                properties.push(SERVER_REFERENCE_IDENTIFIER);
                properties.extend(bytes);

                remaining_length += len;
            } else {
                warn!(
                    "Omitting Server Reference due to size constraints ({} > {})",
                    remaining_length + len,
                    protocol::MAX_ALLOWED_LENGTH
                );
            }
        }

        // Ensure the variable header isn't larger than the MAX_ALLOWED_LENGTH
        if remaining_length > protocol::MAX_ALLOWED_LENGTH {
            error!(
            "Remaining length exceeds the maximum allowed value. Remaining length: {}, Maximum allowed: {}",
            remaining_length, protocol::MAX_ALLOWED_LENGTH
        );
            anyhow::bail!(
            "Remaining length exceeds the maximum allowed value. Remaining length: {}, Maximum allowed: {}",
            remaining_length, protocol::MAX_ALLOWED_LENGTH
        );
        }

        // Add Reason String if present
        if let Some(reason_string) = &self.reason_string {
            let bytes = reason_string.as_bytes();
            let len = 1 + bytes.len();

            // The sender MUST NOT send this Property if it would increase the size of the DISCONNECT packet beyond the Maximum Packet Size specified by the receiver.
            if remaining_length + len <= protocol::MAX_ALLOWED_LENGTH {
                properties.push(REASON_STRING_IDENTIFIER);
                properties.extend(bytes);

                remaining_length += len;
            } else {
                warn!(
                    "Omitting Reason String due to size constraints ({} > {})",
                    remaining_length + len,
                    protocol::MAX_ALLOWED_LENGTH
                );
            }
        }

        // Add User Properties if present
        if let Some(user_properties) = &self.user_properties {
            let mut user_property = Vec::new();

            // Each user property:
            // - 1 byte for identifier
            // - variable length for "name:value" string
            for (name, value) in user_properties {
                user_property.push(USER_PROPERTY_IDENTIFIER);
                user_property.extend(format!("{name}:{value}").as_bytes());
            }

            // The sender MUST NOT send this property if it would increase the size of the DISCONNECT packet beyond the Maximum Packet Size specified by the receiver.
            if user_property.len() <= protocol::MAX_ALLOWED_LENGTH {
                remaining_length += user_property.len();

                properties.extend(user_property);
            } else {
                warn!(
                    "Omitting User Property due to size constraints ({} > {})",
                    user_property.len(),
                    protocol::MAX_ALLOWED_LENGTH
                );
            }
        }

        // According to MQTT spec, variable byte ints should have at most 4 bytes
        let remaining_length = u32::try_from(remaining_length)
            .context("Failed to cast remaining length len to u32")?;
        let properties_length =
            u32::try_from(properties.len()).context("Failed to cast properties length to u32")?;

        // Build packet: Fixed header + Reason code + Property length + Properties
        let mut packet = Vec::new();
        packet.extend(self.create_fixed_header(remaining_length));
        packet.push(self.reason_code as u8);
        packet.extend(protocol::encode_variable_byte_int(properties_length));
        packet.extend(properties);

        Ok(packet)
    }
}
