/// Protocol name.
pub(crate) const PROTOCOL_NAME: &str = "MQTT";

/// Protocol version.
pub(crate) const PROTOCOL_VERSION: u8 = 5;

/// Maximum allowed size for a packet.
pub(crate) const MAX_PACKET_SIZE: usize = 268_435_455;

/// Connection request.
/// Sent by: Client to Server.
pub(crate) const CONNECT_PACKET_TYPE: u8 = 0x01;

/// Connect acknowledgment.
/// Sent by: Server to Client.
pub(crate) const CONNACK_PACKET_TYPE: u8 = 0x02;

/// Publish message.
/// Sent by: Client to Server or Server to Client.
pub(crate) const PUBLISH_PACKET_TYPE: u8 = 0x03;

/// Publish acknowledgment (`QoS` 1).
/// Sent by: Client to Server or Server to Client.
pub(crate) const PUBACK_PACKET_TYPE: u8 = 0x04;

/// Publish received (`QoS` 2 delivery part 1).
/// Sent by: Client to Server or Server to Client.
pub(crate) const PUBREC_PACKET_TYPE: u8 = 0x05;

/// Publish release (`QoS` 2 delivery part 2).
/// Sent by: Client to Server or Server to Client.
pub(crate) const PUBREL_PACKET_TYPE: u8 = 0x06;

/// Publish complete (`QoS` 2 delivery part 3).
/// Sent by: Client to Server or Server to Client.
pub(crate) const PUBCOMP_PACKET_TYPE: u8 = 0x07;

/// Subscribe request.
/// Sent by: Client to Server.
pub(crate) const SUBSCRIBE_PACKET_TYPE: u8 = 0x08;

/// Subscribe acknowledgment.
/// Sent by: Server to Client.
pub(crate) const SUBACK_PACKET_TYPE: u8 = 0x09;

/// Unsubscribe request.
/// Sent by: Client to Server.
pub(crate) const UNSUBSCRIBE_PACKET_TYPE: u8 = 0x0A;

/// Unsubscribe acknowledgment.
/// Sent by: Server to Client.
pub(crate) const UNSUBACK_PACKET_TYPE: u8 = 0x0B;

/// PING request.
/// Sent by: Client to Server.
pub(crate) const PINGREQ_PACKET_TYPE: u8 = 0x0C;

/// PING response.
/// Sent by: Server to Client.
pub(crate) const PINGRESP_PACKET_TYPE: u8 = 0x0D;

/// Disconnect notification.
/// Sent by: Client to Server or Server to Client.
pub(crate) const DISCONNECT_PACKET_TYPE: u8 = 0x0E;

/// Authentication exchange.
/// Sent by: Client to Server or Server to Client.
pub(crate) const AUTH_PACKET_TYPE: u8 = 0x0F;
