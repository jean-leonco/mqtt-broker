use std::fmt;

/// Represents the MQTT Control Packet Types.
#[derive(Debug, Clone)]
pub(crate) enum PacketType {
    /// Connection request.
    /// Sent by: Client to Server.
    Connect = 0x01,

    /// Connect acknowledgment.
    /// Sent by: Server to Client.
    ConnAck = 0x02,

    /// Publish message.
    /// Sent by: Client to Server or Server to Client.
    Publish = 0x03,

    /// Publish acknowledgment (`QoS` 1).
    /// Sent by: Client to Server or Server to Client.
    PubAck = 0x04,

    /// Publish received (`QoS` 2 delivery part 1).
    /// Sent by: Client to Server or Server to Client.
    PubRec = 0x05,

    /// Publish release (`QoS` 2 delivery part 2).
    /// Sent by: Client to Server or Server to Client.
    PubRel = 0x06,

    /// Publish complete (`QoS` 2 delivery part 3).
    /// Sent by: Client to Server or Server to Client.
    PubComp = 0x07,

    /// Subscribe request.
    /// Sent by: Client to Server.
    Subscribe = 0x08,

    /// Subscribe acknowledgment.
    /// Sent by: Server to Client.
    SubAck = 0x09,

    /// Unsubscribe request.
    /// Sent by: Client to Server.
    Unsubscribe = 0x0A,

    /// Unsubscribe acknowledgment.
    /// Sent by: Server to Client.
    UnsubAck = 0x0B,

    /// PING request.
    /// Sent by: Client to Server.
    PingReq = 0x0C,

    /// PING response.
    /// Sent by: Server to Client.
    PingResp = 0x0D,

    /// Disconnect notification.
    /// Sent by: Client to Server or Server to Client.
    Disconnect = 0x0E,

    /// Authentication exchange.
    /// Sent by: Client to Server or Server to Client.
    Auth = 0x0F,
}

impl PacketType {
    /// Converts a numeric value to an `PacketType`.
    ///
    /// Returns `None` if the value does not match a known type.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Connect),
            0x02 => Some(Self::ConnAck),
            0x03 => Some(Self::Publish),
            0x04 => Some(Self::PubAck),
            0x05 => Some(Self::PubRec),
            0x06 => Some(Self::PubRel),
            0x07 => Some(Self::PubComp),
            0x08 => Some(Self::Subscribe),
            0x09 => Some(Self::SubAck),
            0x0A => Some(Self::Unsubscribe),
            0x0B => Some(Self::UnsubAck),
            0x0C => Some(Self::PingReq),
            0x0D => Some(Self::PingResp),
            0x0E => Some(Self::Disconnect),
            0x0F => Some(Self::Auth),
            _ => None,
        }
    }

    /// Converts the `PacketType` to its numeric value.
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Computes the control byte for the MQTT fixed header.
    ///
    /// The control byte is the first byte of the MQTT fixed header, consisting of:
    /// - The packet type (4 most significant bits)
    /// - Packet flags (4 least significant bits)
    ///
    /// This function handles static flags for most packet types as per the MQTT 5.0 specification.
    /// For the `Publish` packet type, dynamic flags such as `QoS`, retain, and DUP are excluded,
    /// as they must be handled separately.
    ///
    /// # Fixed Header Format
    ///
    /// | Bit       | 7   | 6   | 5   | 4   | 3   | 2   | 1   | 0   |
    /// |-----------|-----|-----|-----|-----|-----|-----|-----|-----|
    /// | Byte 1    | Packet type           | Packet flags          |
    /// | Byte 2    | Remaining Length                              |
    pub fn control_byte(self) -> u8 {
        match self {
            // For these packets, the 4 LSB are reserved and must be: 0000
            Self::Connect
            | Self::ConnAck
            | Self::PubAck
            | Self::PubRec
            | Self::PubComp
            | Self::SubAck
            | Self::UnsubAck
            | Self::PingReq
            | Self::PingResp
            | Self::Disconnect
            | Self::Auth
            | Self::Publish => {
                // Shift packet type to MSB. For example, with Connect it becomes: 0000_0001 -> 0001_0000
                // Publish allows dynamic flags, but this function does not handle them
                self.to_u8() << 4
            }

            // For these packets, the 4 LSB are reserved and must be: 0010
            Self::PubRel | Self::Subscribe | Self::Unsubscribe => {
                // Shift packet type to MSB and mask it. For example, with PubRel it becomes: 0000_0110 -> 0110_0010
                self.to_u8() << 4 | 0b0000_0010
            }
        }
    }
}

impl fmt::Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Connect => "CONNECT",
            Self::ConnAck => "CONNACK",
            Self::Publish => "PUBLISH",
            Self::PubAck => "PUBACK",
            Self::PubRec => "PUBREC",
            Self::PubRel => "PUBREL",
            Self::PubComp => "PUBCOMP",
            Self::Subscribe => "SUBSCRIBE",
            Self::SubAck => "SUBACK",
            Self::Unsubscribe => "UNSUBSCRIBE",
            Self::UnsubAck => "UNSUBACK",
            Self::PingReq => "PINGREQ",
            Self::PingResp => "PINGRESP",
            Self::Disconnect => "DISCONNECT",
            Self::Auth => "AUTH",
        };

        write!(f, "{value}")
    }
}
