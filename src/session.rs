use std::time::Duration;

use log::{debug, error, info, trace, warn};
use tokio::{sync::mpsc, time::Instant};

use crate::{
    broker_state::{BrokerEvent, BrokerState},
    connection::{Connection, ConnectionError},
    constants,
    packets::{
        conn_ack_packet::{ConnAckPacket, ConnectReasonCode},
        connect_packet::ConnectPacket,
        disconnect_packet::{DisconnectPacket, DisconnectReasonCode},
        ping_req_packet::PingReqPacket,
        ping_resp_packet::PingRespPacket,
        publish_packet::PublishPacket,
        subscribe_packet::SubscribePacket,
        CommonPacketError,
    },
};

use self::constants::{
    AUTH_PACKET_TYPE, CONNACK_PACKET_TYPE, CONNECT_PACKET_TYPE, DISCONNECT_PACKET_TYPE,
    PINGREQ_PACKET_TYPE, PINGRESP_PACKET_TYPE, PUBACK_PACKET_TYPE, PUBCOMP_PACKET_TYPE,
    PUBLISH_PACKET_TYPE, PUBREC_PACKET_TYPE, PUBREL_PACKET_TYPE, SUBACK_PACKET_TYPE,
    SUBSCRIBE_PACKET_TYPE, UNSUBACK_PACKET_TYPE, UNSUBSCRIBE_PACKET_TYPE,
};

pub(crate) struct Session {
    connection: Connection,
    broker_state: BrokerState,
    client_id: String,
    keep_alive: Duration,
    expiry_interval: Option<Duration>,
    last_activity: Instant,
}

impl Session {
    pub(crate) async fn handle_connection(
        mut connection: Connection,
        mut broker_state: BrokerState,
    ) -> Result<(), ConnectionError> {
        debug!("Handling new connection");

        let connect = connection
            .read_packet::<ConnectPacket>()
            .await?
            .ok_or(ConnectionError::Common(CommonPacketError::ProtocolError(None)))?;

        let (session_present, rx) =
            broker_state.save_session(connect.client_id.clone(), connect.clean_start);

        info!("Client {} connected with clean_start={}", connect.client_id, connect.clean_start);
        debug!("Session present: {}", session_present);

        let keep_alive = Duration::from_secs(u64::from(connect.keep_alive));

        let expiry_interval = connect
            .properties
            .session_expiry_interval
            .map(|expiry_interval| Duration::from_secs(u64::from(expiry_interval)));

        debug!(
            "Client settings - keep_alive: {:?}, expiry_interval: {:?}",
            keep_alive, expiry_interval
        );

        let mut session = Self {
            connection,
            broker_state,
            client_id: connect.client_id,
            keep_alive,
            expiry_interval,
            last_activity: Instant::now(),
        };

        let response = ConnAckPacket::builder()
            .session_present(session_present)
            .reason_code(ConnectReasonCode::Success)
            .build();

        session.connection.write_packet(&response).await?;
        debug!("Sent CONNACK to client {}", session.client_id);

        if let Err(e) = session.run(rx).await {
            session.discard_session();
            error!("Session for client {} ended with error: {:?}", session.client_id, e);
            return Err(e);
        }

        Ok(())
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<BrokerEvent>) -> Result<(), ConnectionError> {
        let timeout = self.keep_alive.mul_f64(1.5);
        debug!("Starting session loop for client {} with timeout {:?}", self.client_id, timeout);

        loop {
            let deadline = self.last_activity + timeout;

            tokio::select! {
                // Handle broker events
                event = rx.recv() => {
                    match event {
                        Some(event) => {
                          debug!("Client {} received broker event: {:?}", self.client_id, event);
                        }
                        None => {
                            // The channel is closed, session can exit
                            return Ok(());
                        }
                    }
                },

                // Handle incoming packets
                packet_result = self.handle_incoming_packet() => {
                    let should_disconnect = packet_result?;

                    if should_disconnect {
                        info!("Client {} disconnected", self.client_id);
                        return Ok(());
                    }

                    self.last_activity = Instant::now();
                },

                // Handle keep-alive timeout
                _ = tokio::time::sleep_until(deadline) => {
                    warn!("Keep-alive timeout for client {}", self.client_id);
                    return Ok(());
                }
            }
        }
    }

    async fn handle_incoming_packet(&mut self) -> Result<bool, ConnectionError> {
        match self.connection.peek_packet_type().await? {
            None => {
                debug!("Connection reset by client {}", self.client_id);
                return Ok(true);
            }

            Some(packet_type) => match packet_type {
                PUBACK_PACKET_TYPE
                | PUBREC_PACKET_TYPE
                | PUBREL_PACKET_TYPE
                | PUBCOMP_PACKET_TYPE
                | UNSUBSCRIBE_PACKET_TYPE
                | AUTH_PACKET_TYPE => todo!("Not implemented: {packet_type}"),

                SUBSCRIBE_PACKET_TYPE => {
                    let subscribe =
                        self.connection.read_packet::<SubscribePacket>().await?.unwrap();

                    debug!(
                        "Client {} subscribing to {} topics",
                        self.client_id,
                        subscribe.topics.len()
                    );

                    for topic in subscribe.topics {
                        trace!("Client {} subscribing to topic {}", self.client_id, topic.topic);
                        self.broker_state.subscribe(topic.topic, &self.client_id);
                    }
                }

                PUBLISH_PACKET_TYPE => {
                    let publish = self.connection.read_packet::<PublishPacket>().await?.unwrap();
                    self.broker_state.publish(&publish.topic, publish.payload);

                    trace!("Client {} published message", self.client_id);
                }

                PINGREQ_PACKET_TYPE => {
                    self.connection.read_packet::<PingReqPacket>().await?.unwrap();
                    self.connection.write_packet(&PingRespPacket {}).await?;
                }

                DISCONNECT_PACKET_TYPE => {
                    let disconnect =
                        self.connection.read_packet::<DisconnectPacket>().await?.unwrap();

                    info!(
                        "Client {} sent DISCONNECT with reason code: {}",
                        self.client_id, disconnect.reason_code
                    );

                    return Ok(false);
                }

                CONNECT_PACKET_TYPE => {
                    let disconnect = DisconnectPacket::builder()
                            .reason_code(DisconnectReasonCode::ProtocolError)
                            .reason_string("A Client can only send the CONNECT packet once over a Network Connection".into())
                            .build();

                    warn!("Client {} sent unexpected CONNECT packet", self.client_id);
                    self.connection.write_packet(&disconnect).await?;
                    return Ok(false);
                }

                CONNACK_PACKET_TYPE | SUBACK_PACKET_TYPE | UNSUBACK_PACKET_TYPE
                | PINGRESP_PACKET_TYPE => {
                    // Packets reserved to server -> client communication
                    let disconnect = DisconnectPacket::builder()
                        .reason_code(DisconnectReasonCode::ProtocolError)
                        .reason_string(format!("Packet {packet_type} is reserved for Server use"))
                        .build();

                    warn!(
                        "Client {} sent server-to-client packet type: {}",
                        self.client_id, packet_type
                    );
                    self.connection.write_packet(&disconnect).await?;
                    return Ok(false);
                }

                _ => {
                    let disconnect = DisconnectPacket::builder()
                        .reason_code(DisconnectReasonCode::ProtocolError)
                        .build();

                    warn!("Client {} sent unknown packet type: {}", self.client_id, packet_type);
                    self.connection.write_packet(&disconnect).await?;
                    return Ok(false);
                }
            },
        };

        Ok(false)
    }

    fn discard_session(&mut self) {
        if let Some(expiry_interval) = self.expiry_interval {
            self.broker_state
                .schedule_discard_session(&self.client_id, Instant::now() + expiry_interval);
        }
        info!("Discarding session for client {}", self.client_id);
    }
}
