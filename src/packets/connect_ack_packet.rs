/// MQTT Connection Reason Codes
///
/// This enum represents the possible reason codes returned when attempting
/// to connect to an MQTT server. Each variant corresponds to a specific
/// connection outcome or error.
#[derive(Debug)]
enum ConnectReasonCode {
    /// 0x00: Success
    /// The Connection is accepted.
    Success = 0x00,

    /// 0x80: Unspecified error
    /// The Server does not wish to reveal the reason for the failure,
    /// or none of the other Reason Codes apply.
    UnspecifiedError = 0x80,

    /// 0x81: Malformed Packet
    /// Data within the CONNECT packet could not be correctly parsed.
    MalformedPacket = 0x81,

    /// 0x82: Protocol Error
    /// Data in the CONNECT packet does not conform to this specification.
    ProtocolError = 0x82,

    /// 0x83: Implementation specific error
    /// The CONNECT is valid but is not accepted by this Server.
    ImplementationSpecificError = 0x83,

    /// 0x84: Unsupported Protocol Version
    /// The Server does not support the version of the MQTT protocol requested by the Client.
    UnsupportedProtocolVersion = 0x84,

    /// 0x85: Client Identifier not valid
    /// The Client Identifier is a valid string but is not allowed by the Server.
    ClientIdentifierNotValid = 0x85,

    /// 0x86: Bad User Name or Password
    /// The Server does not accept the User Name or Password specified by the Client.
    BadUserNameOrPassword = 0x86,

    /// 0x87: Not authorized
    /// The Client is not authorized to connect.
    NotAuthorized = 0x87,

    /// 0x88: Server unavailable
    /// The MQTT Server is not available.
    ServerUnavailable = 0x88,

    /// 0x89: Server busy
    /// The Server is busy. Try again later.
    ServerBusy = 0x89,

    /// 0x8A: Banned
    /// This Client has been banned by administrative action. Contact the server administrator.
    Banned = 0x8A,

    /// 0x8C: Bad authentication method
    /// The authentication method is not supported or does not match the authentication method currently in use.
    BadAuthenticationMethod = 0x8C,

    /// 0x90: Topic Name invalid
    /// The Will Topic Name is not malformed, but is not accepted by this Server.
    TopicNameInvalid = 0x90,

    /// 0x95: Packet too large
    /// The CONNECT packet exceeded the maximum permissible size.
    PacketTooLarge = 0x95,

    /// 0x97: Quota exceeded
    /// An implementation or administrative imposed limit has been exceeded.
    QuotaExceeded = 0x97,

    /// 0x99: Payload format invalid
    /// The Will Payload does not match the specified Payload Format Indicator.
    PayloadFormatInvalid = 0x99,

    /// 0x9A: Retain not supported
    /// The Server does not support retained messages, and Will Retain was set to 1.
    RetainNotSupported = 0x9A,

    /// 0x9B: QoS not supported
    /// The Server does not support the QoS set in Will QoS.
    QoSNotSupported = 0x9B,

    /// 0x9C: Use another server
    /// The Client should temporarily use another server.
    UseAnotherServer = 0x9C,

    /// 0x9D: Server moved
    /// The Client should permanently use another server.
    ServerMoved = 0x9D,

    /// 0x9F: Connection rate exceeded
    /// The connection rate limit has been exceeded.
    ConnectionRateExceeded = 0x9F,
}

#[derive(Debug)]
enum MaximumQos {
    ZERO = 0,
    ONE = 1,
}

#[derive(Debug)]
struct ConnAckPacket {
    /// The Session Present flag informs the Client whether the Server is using Session State from a previous connection for this ClientID. This allows the Client and Server to have a consistent view of the Session State.
    session_preset: bool,
    /// If a well formed CONNECT packet is received by the Server, but the Server is unable to complete the Connection the Server MAY send a CONNACK packet containing the appropriate Connect Reason code. If a Server sends a CONNACK packet containing a Reason code of 128 or greater it MUST then close the Network Connection.
    reason_code: ConnectReasonCode,
    /// Session Expiry Interval in seconds. If the Session Expiry Interval is absent the value in the CONNECT Packet used. The server uses this property to inform the Client that it is using a value other than that sent by the Client in the CONNACK.
    session_expiry_interval: Option<u32>,
    /// The Server uses this value to limit the number of QoS 1 and QoS 2 publications that it is willing to process concurrently for the Client. It does not provide a mechanism to limit the QoS 0 publications that the Client might try to send.
    receive_maximum: Option<u16>,
    /// If a Server does not support QoS 1 or QoS 2 PUBLISH packets it MUST send a Maximum QoS in the CONNACK packet specifying the highest QoS it supports.
    maximum_qos: Option<MaximumQos>,
    /// If present, this byte declares whether the Server supports retained messages. A value of 0 means that retained messages are not supported. A value of 1 means retained messages are supported. If not present, then retained messages are supported.
    retain_available: Option<bool>,
    /// Representing the Maximum Packet Size the Server is willing to accept. If the Maximum Packet Size is not present, there is no limit on the packet size imposed beyond the limitations in the protocol as a result of the remaining length encoding and the protocol header sizes. It is a Protocol Error to include the Maximum Packet Size more than once, or for the value to be set to zero.
    maximum_packet_size: Option<u32>,
}

impl ConnAckPacket {
    fn to_bytes(self) {}
}
