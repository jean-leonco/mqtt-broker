use bytes::{Buf, BufMut, BytesMut};
use std::{collections::HashMap, fmt, io::Cursor};

use crate::{
    codec::{
        decode_variable_byte_int, encode_variable_byte_int, write_usize_as_var_int,
        write_utf8_string, write_utf8_string_pair,
    },
    constants::{DISCONNECT_PACKET_TYPE, MAX_PACKET_SIZE},
};

use super::{CommonPacketError, DecodablePacket, EncodablePacket, Packet};

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
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::NormalDisconnection),
            0x04 => Some(Self::DisconnectWithWillMessage),
            0x80 => Some(Self::UnspecifiedError),
            0x81 => Some(Self::MalformedPacket),
            0x82 => Some(Self::ProtocolError),
            0x83 => Some(Self::ImplementationSpecificError),
            0x87 => Some(Self::NotAuthorized),
            0x89 => Some(Self::ServerBusy),
            0x8B => Some(Self::ServerShuttingDown),
            0x8D => Some(Self::KeepAliveTimeout),
            0x8E => Some(Self::SessionTakenOver),
            0x8F => Some(Self::TopicFilterInvalid),
            0x90 => Some(Self::TopicNameInvalid),
            0x93 => Some(Self::ReceiveMaximumExceeded),
            0x94 => Some(Self::TopicAliasInvalid),
            0x95 => Some(Self::PacketTooLarge),
            0x96 => Some(Self::MessageRateTooHigh),
            0x97 => Some(Self::QuotaExceeded),
            0x98 => Some(Self::AdministrativeAction),
            0x99 => Some(Self::PayloadFormatInvalid),
            0x9A => Some(Self::RetainNotSupported),
            0x9B => Some(Self::QosNotSupported),
            0x9C => Some(Self::UseAnotherServer),
            0x9D => Some(Self::ServerMoved),
            0x9E => Some(Self::SharedSubscriptionsNotSupported),
            0x9F => Some(Self::ConnectionRateExceeded),
            0xA0 => Some(Self::MaximumConnectTime),
            0xA1 => Some(Self::SubscriptionIdentifiersNotSupported),
            0xA2 => Some(Self::WildcardSubscriptionsNotSupported),
            _ => None,
        }
    }

    fn to_u8(self) -> u8 {
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
#[derive(Debug)]
pub(crate) struct DisconnectPacket {
    /// The Reason Code indicating why the DISCONNECT is occurring.
    pub reason_code: DisconnectReasonCode,

    /// Disconnect properties.
    properties: DisconnectPacketProperties,
}

#[derive(Debug)]
pub(crate) enum DisconnectPacketDecodeError {
    Common(CommonPacketError),
}

impl std::error::Error for DisconnectPacketDecodeError {}

impl fmt::Display for DisconnectPacketDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Common(e) => write!(f, "Common Error: {e}"),
        }
    }
}

impl Packet for DisconnectPacket {
    fn packet_type() -> u8 {
        DISCONNECT_PACKET_TYPE
    }
}

impl DecodablePacket for DisconnectPacket {
    type Error = DisconnectPacketDecodeError;

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
        //  if the remaining length is less than 1, normal disconnection can be assumed
        if cursor.remaining() == 0 {
            return Ok(Self {
                reason_code: DisconnectReasonCode::NormalDisconnection,
                properties: DisconnectPacketProperties {
                    session_expiry_interval: None,
                    reason_string: None,
                    user_properties: None,
                    server_reference: None,
                },
            });
        }

        let raw_reason_code = cursor.get_u8();
        let Some(reason_code) = DisconnectReasonCode::from_u8(raw_reason_code) else {
            let e = CommonPacketError::MalformedPacket(Some(format!(
                "Reason code is not a valid reason code: {raw_reason_code}"
            )));
            return Err(Self::Error::Common(e));
        };

        let properties_len = decode_variable_byte_int(cursor).map_err(Self::Error::Common)?;

        let properties = DisconnectPacketProperties::decode(cursor, properties_len)?;

        Ok(Self { reason_code, properties })
    }
}

#[derive(Debug)]
pub(crate) enum DisconnectPacketEncodeError {
    Common(CommonPacketError),
}

impl std::error::Error for DisconnectPacketEncodeError {}

impl fmt::Display for DisconnectPacketEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Common(e) => write!(f, "Common Error: {e}"),
        }
    }
}

impl EncodablePacket for DisconnectPacket {
    type Error = DisconnectPacketEncodeError;

    fn encode(&self) -> Result<bytes::BytesMut, Self::Error> {
        let mut properties_buf = BytesMut::new();
        self.properties.write_properties(&mut properties_buf, self.reason_code)?;

        let properties_len = encode_variable_byte_int(
            u32::try_from(properties_buf.len())
                .map_err(|_| Self::Error::Common(CommonPacketError::IntOverflow))?,
        );

        let mut buf = BytesMut::new();

        // Fixed header
        buf.put_u8(Self::packet_type() << 4);

        // Remaining length: reason code + properties len + properties
        let remaining_len = 1 + properties_buf.len() + properties_buf.len();
        write_usize_as_var_int(&mut buf, remaining_len).map_err(Self::Error::Common)?;

        // Reason code
        buf.put_u8(self.reason_code.to_u8());

        // Properties
        buf.put(&properties_len[..]);
        buf.put(properties_buf);

        if buf.len() > MAX_PACKET_SIZE {
            return Err(Self::Error::Common(CommonPacketError::PacketTooLarge(None)));
        }

        Ok(buf)
    }
}

impl DisconnectPacket {
    pub(crate) fn builder() -> DisconnectPacketBuilder<NeedsReasonCode> {
        DisconnectPacketBuilder(NeedsReasonCode(()))
    }
}

pub struct DisconnectPacketBuilder<State>(State);

pub struct NeedsReasonCode(());

pub struct ReadyToBuild {
    reason_code: DisconnectReasonCode,
    reason_string: Option<String>,
}

impl DisconnectPacketBuilder<NeedsReasonCode> {
    pub(crate) fn reason_code(
        self,
        reason_code: DisconnectReasonCode,
    ) -> DisconnectPacketBuilder<ReadyToBuild> {
        DisconnectPacketBuilder(ReadyToBuild { reason_code, reason_string: None })
    }
}

impl DisconnectPacketBuilder<ReadyToBuild> {
    pub(crate) fn reason_string(
        self,
        reason_string: String,
    ) -> DisconnectPacketBuilder<ReadyToBuild> {
        DisconnectPacketBuilder(ReadyToBuild { reason_string: Some(reason_string), ..self.0 })
    }

    pub(crate) fn build(self) -> DisconnectPacket {
        DisconnectPacket {
            reason_code: self.0.reason_code,
            properties: DisconnectPacketProperties {
                session_expiry_interval: None,
                reason_string: self.0.reason_string,
                user_properties: None,
                server_reference: None,
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct DisconnectPacketProperties {
    /// Represents the Session Expiry Interval in seconds.
    /// The Session Expiry Interval MUST NOT be sent on a DISCONNECT by the Server.
    session_expiry_interval: Option<u32>,

    /// A human-readable reason string for diagnostic purposes. Should NOT be parsed programmatically.
    reason_string: Option<String>,

    /// User properties in key-value pairs.
    user_properties: Option<HashMap<String, String>>,

    /// Encoded String which can be used by the Client to identify another Server to use.
    server_reference: Option<String>,
}

impl DisconnectPacketProperties {
    fn decode(
        cursor: &mut Cursor<&[u8]>,
        len: usize,
    ) -> Result<DisconnectPacketProperties, DisconnectPacketDecodeError> {
        if len > 0 {
            cursor.advance(len);
        }

        Ok(DisconnectPacketProperties {
            session_expiry_interval: None,
            reason_string: None,
            user_properties: None,
            server_reference: None,
        })
    }

    fn write_properties(
        &self,
        buf: &mut BytesMut,
        reason_code: DisconnectReasonCode,
    ) -> Result<(), DisconnectPacketEncodeError> {
        // Check if we have any properties at all
        let has_properties = self.session_expiry_interval.is_some()
            || self.reason_string.is_some()
            || self.user_properties.is_some()
            || self.server_reference.is_some();

        // If reason code is NormalDisconnection (0x00) and no properties, we can omit Reason Code and Property Length.
        if matches!(reason_code, DisconnectReasonCode::NormalDisconnection) && !has_properties {
            return Ok(());
        }

        if let Some(session_expiry_interval) = self.session_expiry_interval {
            buf.put_u8(SESSION_EXPIRY_INTERVAL_IDENTIFIER);
            buf.put_u32(session_expiry_interval);
        }

        if let Some(server_reference) = &self.server_reference {
            buf.put_u8(SERVER_REFERENCE_IDENTIFIER);
            write_utf8_string(buf, server_reference)
                .map_err(DisconnectPacketEncodeError::Common)?;
        }

        if let Some(reason_string) = &self.reason_string {
            buf.put_u8(REASON_STRING_IDENTIFIER);
            write_utf8_string(buf, reason_string).map_err(DisconnectPacketEncodeError::Common)?;
        }

        if let Some(user_properties) = &self.user_properties {
            for user_property in user_properties {
                buf.put_u8(USER_PROPERTY_IDENTIFIER);
                write_utf8_string_pair(buf, user_property)
                    .map_err(DisconnectPacketEncodeError::Common)?;
            }
        }

        Ok(())
    }
}
