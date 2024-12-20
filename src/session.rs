use std::{fmt, time::Duration};

use log::{debug, error};
use tokio::{io, sync::mpsc, time::Instant};

use crate::{
    broker_state::{BrokerEvent, BrokerState},
    connection::{Connection, IncomingPacket, OutgoingPacket, PacketError},
    packets::{
        conn_ack_packet::{ConnAckPacket, ConnectReasonCode},
        disconnect_packet::{DisconnectPacket, DisconnectReasonCode},
    },
};

pub(crate) struct Session {
    connection: Connection,
    rx: mpsc::Receiver<BrokerEvent>,
    broker_state: BrokerState,
    client_id: String,
    keep_alive: Duration,
    expiry_interval: Option<Duration>,
}

#[derive(Debug)]
pub(crate) enum SessionError {
    PacketError(PacketError),
    IoError(io::Error),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PacketError(packet_error) => write!(f, "Packet Error: {packet_error}"),
            Self::IoError(error) => write!(f, "IO Error: {error}"),
        }
    }
}

impl Session {
    pub async fn handle_connection(
        mut connection: Connection,
        mut broker_state: BrokerState,
    ) -> Result<(), SessionError> {
        let first_packet = connection.read_packet().await.map_err(SessionError::PacketError)?;

        let mut session = match first_packet {
            Some(IncomingPacket::Connect(packet)) => {
                let (session_present, rx) =
                    broker_state.save_session(packet.client_id.clone(), packet.clean_start);

                let response = ConnAckPacket::builder()
                    .session_present(session_present)
                    .reason_code(ConnectReasonCode::Success)
                    .build();
                connection
                    .write_packet(OutgoingPacket::ConnAck(response))
                    .await
                    .map_err(SessionError::IoError)?;

                let keep_alive = Duration::from_secs(u64::from(packet.keep_alive));

                let expiry_interval = packet
                    .session_expiry_interval
                    .map(|expiry_interval| Duration::from_secs(u64::from(expiry_interval)));

                Self {
                    connection,
                    rx,
                    broker_state,
                    client_id: packet.client_id,
                    keep_alive,
                    expiry_interval,
                }
            }
            Some(_) => {
                error!("First packet was not CONNECT");
                return Err(SessionError::PacketError(PacketError::ProtocolError));
            }
            None => {
                error!("No packet received");
                return Err(SessionError::PacketError(PacketError::ProtocolError));
            }
        };

        if let Err(e) = session.run().await {
            // Discard session regardless of an error
            session.discard_session();

            return Err(e);
        }

        Ok(())
    }

    async fn run(&mut self) -> Result<(), SessionError> {
        let mut last_activity = Instant::now();

        loop {
            tokio::select! {
                packet = self.connection.read_packet() => {
                    match packet.map_err(SessionError::PacketError)? {
                        Some(packet) => {
                            last_activity = Instant::now();

                            match packet {
                                IncomingPacket::Connect(_) => {
                                    let packet = DisconnectPacket::builder()
                                        .reason_code(DisconnectReasonCode::ProtocolError)
                                        .reason_string("A Client can only send the CONNECT packet once over a Network Connection.".to_string())
                                        .build();

                                    self.connection
                                        .write_packet(OutgoingPacket::Disconnect(packet))
                                        .await
                                        .map_err(SessionError::IoError)?;

                                    return Ok(());
                                }
                                IncomingPacket::Subscribe(subscribe_packet) => {
                                    for topic in subscribe_packet.topics {
                                        self.broker_state.subscribe(topic.name, &self.client_id);
                                    }
                                },
                                IncomingPacket::Publish => {
                                    self.broker_state.publish("some/topic");
                                }
                                IncomingPacket::PingReq => {
                                    self.connection
                                        .write_packet(OutgoingPacket::PingResp)
                                        .await
                                        .map_err(SessionError::IoError)?;
                                }
                                IncomingPacket::Disconnect(_) => {
                                    return Ok(());
                                }
                            }
                        }
                        None => {
                            if self.keep_alive > Duration::from_secs(0)
                                && Instant::now().duration_since(last_activity)
                                    > self.keep_alive.mul_f64(1.5)
                            {
                                return Ok(());
                            }
                        }
                    }
                }
                event = self.rx.recv() => {
                    debug!("Received event {:?}", event);
                }
            }
        }
    }

    fn discard_session(&mut self) {
        if let Some(expiry_interval) = self.expiry_interval {
            self.broker_state
                .schedule_discard_session(&self.client_id, Instant::now() + expiry_interval);
        }
    }
}
