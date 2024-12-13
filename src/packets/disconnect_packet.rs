use std::{collections::HashMap, fmt};

use crate::protocol;

use self::protocol::MAX_ALLOWED_LENGTH;

const SESSION_EXPIRY_INTERVAL_IDENTIFIER: u8 = 0x11;
const REASON_STRING_IDENTIFIER: u8 = 0x1F;
const USER_PROPERTY_IDENTIFIER: u8 = 0x26;
const SERVER_REFERENCE_IDENTIFIER: u8 = 0x1C;

/// This enum represents the possible reason codes sent by the Client or Server when
/// disconnecting from an MQTT connection.
#[derive(Debug)]
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

    /// The Connection is closed because no packet has been received for 1.5 times the Keepalive time.
    /// Sent by: Server.
    KeepAliveTimeout = 0x8D,

    /// Another Connection using the same ClientID has connected causing this Connection to be closed.
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

    /// The Client specified a QoS greater than the QoS specified in a Maximum QoS in the CONNACK.
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
        match self {
            DisconnectReasonCode::NormalDisconnection => write!(f, "NormalDisconnection"),
            DisconnectReasonCode::DisconnectWithWillMessage => {
                write!(f, "DisconnectWithWillMessage")
            }
            DisconnectReasonCode::UnspecifiedError => write!(f, "UnspecifiedError"),
            DisconnectReasonCode::MalformedPacket => write!(f, "MalformedPacket"),
            DisconnectReasonCode::ProtocolError => write!(f, "ProtocolError"),
            DisconnectReasonCode::ImplementationSpecificError => {
                write!(f, "ImplementationSpecificError")
            }
            DisconnectReasonCode::NotAuthorized => write!(f, "NotAuthorized"),
            DisconnectReasonCode::ServerBusy => write!(f, "ServerBusy"),
            DisconnectReasonCode::ServerShuttingDown => write!(f, "ServerShuttingDown"),
            DisconnectReasonCode::KeepAliveTimeout => write!(f, "KeepAliveTimeout"),
            DisconnectReasonCode::SessionTakenOver => write!(f, "SessionTakenOver"),
            DisconnectReasonCode::TopicFilterInvalid => write!(f, "TopicFilterInvalid"),
            DisconnectReasonCode::TopicNameInvalid => write!(f, "TopicNameInvalid"),
            DisconnectReasonCode::ReceiveMaximumExceeded => write!(f, "ReceiveMaximumExceeded"),
            DisconnectReasonCode::TopicAliasInvalid => write!(f, "TopicAliasInvalid"),
            DisconnectReasonCode::PacketTooLarge => write!(f, "PacketTooLarge"),
            DisconnectReasonCode::MessageRateTooHigh => write!(f, "MessageRateTooHigh"),
            DisconnectReasonCode::QuotaExceeded => write!(f, "QuotaExceeded"),
            DisconnectReasonCode::AdministrativeAction => write!(f, "AdministrativeAction"),
            DisconnectReasonCode::PayloadFormatInvalid => write!(f, "PayloadFormatInvalid"),
            DisconnectReasonCode::RetainNotSupported => write!(f, "RetainNotSupported"),
            DisconnectReasonCode::QoSNotSupported => write!(f, "QoSNotSupported"),
            DisconnectReasonCode::UseAnotherServer => write!(f, "UseAnotherServer"),
            DisconnectReasonCode::ServerMoved => write!(f, "ServerMoved"),
            DisconnectReasonCode::SharedSubscriptionsNotSupported => {
                write!(f, "SharedSubscriptionsNotSupported")
            }
            DisconnectReasonCode::ConnectionRateExceeded => write!(f, "ConnectionRateExceeded"),
            DisconnectReasonCode::MaximumConnectTime => write!(f, "MaximumConnectTime"),
            DisconnectReasonCode::SubscriptionIdentifiersNotSupported => {
                write!(f, "SubscriptionIdentifiersNotSupported")
            }
            DisconnectReasonCode::WildcardSubscriptionsNotSupported => {
                write!(f, "WildcardSubscriptionsNotSupported")
            }
        }
    }
}

/// The DISCONNECT packet is the final MQTT Control Packet sent from the Client or the Server. It indicates the reason why the Network Connection is being closed. The Client or Server MAY send a DISCONNECT packet before closing the Network Connection. If the Network Connection is closed without the Client first sending a DISCONNECT packet with Reason Code 0x00 (Normal disconnection) and the Connection has a Will Message, the Will Message is published.
#[derive(Debug)]
pub(crate) struct DisconnectPacket {
    /// The Client or Server sending the DISCONNECT packet MUST use one of the DISCONNECT Reason Code values.
    reason_code: DisconnectReasonCode,
    /// Representis the Session Expiry Interval in seconds.
    /// The Session Expiry Interval MUST NOT be sent on a DISCONNECT by the Server.
    session_expiral_interval: Option<u32>,
    /// Encoded String representing the reason for the disconnect. This Reason String is human readable, designed for diagnostics and SHOULD NOT be parsed by the receiver.
    reason_string: Option<String>,
    /// String Pair. This property may be used to provide additional diagnostic or other information.
    user_properties: Option<HashMap<String, String>>, // TODO: The same name is allowed to appear more than once
    /// Encoded String which can be used by the Client to identify another Server to use.
    server_reference: Option<String>,
}

impl DisconnectPacket {
    pub fn new(
        reason_code: DisconnectReasonCode,
        session_expiral_interval: Option<u32>,
        reason_string: Option<String>,
        user_properties: Option<HashMap<String, String>>,
        server_reference: Option<String>,
    ) -> Self {
        if let Some(server_reference) = server_reference {
            match reason_code {
                DisconnectReasonCode::UseAnotherServer | DisconnectReasonCode::ServerMoved => {
                    return Self {
                        reason_code,
                        session_expiral_interval,
                        reason_string,
                        user_properties,
                        server_reference: Some(server_reference),
                    }
                }
                reason_code => unreachable!(
                    "Expected reason_code to be {} or {}. Got {reason_code}",
                    DisconnectReasonCode::UseAnotherServer,
                    DisconnectReasonCode::ServerMoved
                ),
            }
        }

        Self {
            reason_code,
            session_expiral_interval,
            reason_string,
            user_properties,
            server_reference: None,
        }
    }

    fn get_fixed_header(&self, remaining_length: usize) -> Vec<u8> {
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
                let remaining_length_n_bytes = remaining_length.div_ceil(128);
                let mut header = vec![0; 1 + remaining_length_n_bytes];

                header[0] = protocol::DISCONNECT << 4; // shift code to left (most significant bits)
                header.extend(remaining_length.to_be_bytes());

                header
            }
        }
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let has_properties = self.reason_string.is_some()
            || self.user_properties.is_some()
            || self.server_reference.is_some();

        // The Reason Code and Property Length can be omitted if the Reason Code is 0x00 (Normal disconnecton) and there are no Properties.
        if let DisconnectReasonCode::NormalDisconnection = self.reason_code {
            if !has_properties {
                return Ok(self.get_fixed_header(0));
            }
        }

        let mut remaining_length = 0;
        let mut variable_header = Vec::new(); // TODO: Pre-allocate size

        if let Some(session_expiral_interval) = self.session_expiral_interval {
            variable_header.push(SESSION_EXPIRY_INTERVAL_IDENTIFIER);
            variable_header.extend(session_expiral_interval.to_be_bytes());
            remaining_length += variable_header.len();
        }

        if let Some(server_reference) = &self.server_reference {
            variable_header.push(SERVER_REFERENCE_IDENTIFIER);
            variable_header.extend(server_reference.as_bytes());
            remaining_length += variable_header.len();
        }

        // ensure the variable header isn't larger than the max allowed value
        if remaining_length > MAX_ALLOWED_LENGTH {
            anyhow::bail!(
                "Remaining length ({remaining_length}) exceeds the maximum allowed value ({MAX_ALLOWED_LENGTH})",
            );
        }

        if let Some(reason_string) = &self.reason_string {
            let reason_string_bytes = reason_string.as_bytes();

            // The sender MUST NOT send this Property if it would increase the size of the DISCONNECT packet beyond the Maximum Packet Size specified by the receiver.
            // TODO: Use receiver max packet size instead of MAX_ALLOWED_LENGTH
            if remaining_length + reason_string_bytes.len() <= protocol::MAX_ALLOWED_LENGTH {
                variable_header.push(REASON_STRING_IDENTIFIER);
                variable_header.extend(reason_string_bytes);
                remaining_length += variable_header.len();
            }
        }

        if let Some(user_properties) = &self.user_properties {
            let mut user_property = Vec::new(); // TODO: Pre-allocate size
            for (name, value) in user_properties {
                user_property.push(USER_PROPERTY_IDENTIFIER);
                user_property.extend(format!("{name}:{value}").as_bytes());
            }

            // The sender MUST NOT send this property if it would increase the size of the DISCONNECT packet beyond the Maximum Packet Size specified by the receiver.
            // TODO: Use receiver max packet size instead of MAX_ALLOWED_LENGTH
            if remaining_length + user_property.len() <= protocol::MAX_ALLOWED_LENGTH {
                variable_header.extend(user_property);
                remaining_length += variable_header.len();
            }
        }

        let mut buf = self.get_fixed_header(remaining_length);
        buf.extend(remaining_length.to_be_bytes());
        buf.extend(variable_header);
        Ok(buf)
    }
}
