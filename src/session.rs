use std::time::Duration;

use log::debug;
use tokio::{sync::mpsc, time::Instant};

use crate::{
    broker_state::{BrokerEvent, BrokerState},
    connection::{Connection, IncomingPacket, OutgoingPacket},
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
    session_expiry_interval: Option<Duration>,
}

impl Session {
    pub async fn handle_session(
        mut connection: Connection,
        mut broker_state: BrokerState,
    ) -> anyhow::Result<()> {
        let first_packet = connection.read_packet().await.unwrap();

        let mut session = match first_packet {
            Some(IncomingPacket::Connect(packet)) => {
                let (session_present, rx) =
                    broker_state.save_session(packet.client_id.clone(), packet.clean_start);

                let response = ConnAckPacket::builder()
                    .session_present(session_present)
                    .reason_code(ConnectReasonCode::Success)
                    .build();
                connection.write_packet(OutgoingPacket::ConnAck(response)).await.unwrap();

                let keep_alive = Duration::from_secs(packet.keep_alive as u64);

                let session_expiry_interval =
                    if let Some(session_expiry_interval) = packet.session_expiry_interval {
                        Some(Duration::from_secs(session_expiry_interval as u64))
                    } else {
                        None
                    };

                Self {
                    connection,
                    rx,
                    broker_state,
                    client_id: packet.client_id,
                    keep_alive,
                    session_expiry_interval,
                }
            }
            Some(_) => {
                anyhow::bail!("First packet was not CONNECT");
            }
            None => {
                anyhow::bail!("No packet received");
            }
        };

        session.run().await;

        session.discard_session();

        Ok(())
    }

    async fn run(&mut self) {
        let mut last_activity = Instant::now();

        loop {
            tokio::select! {
                packet = self.connection.read_packet() => {
                    match packet.unwrap() {
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
                                        .unwrap();
                                    break;
                                }
                                IncomingPacket::Subscribe(subscribe_packet) => {
                                    for topic in subscribe_packet.topics {
                                        self.broker_state.subscribe(topic.name, self.client_id.clone());
                                    }
                                },
                                IncomingPacket::Publish => {
                                    self.broker_state.publish("some/topic".to_string());
                                }
                                IncomingPacket::Disconnect(_) => {
                                    break;
                                }
                            }
                        }
                        None => {
                            if self.keep_alive > Duration::from_secs(0)
                                && Instant::now().duration_since(last_activity)
                                    > self.keep_alive.mul_f64(1.5)
                            {
                                break;
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
        if let Some(session_expiry_interval) = self.session_expiry_interval {
            self.broker_state.schedule_discard_session(
                &self.client_id,
                Instant::now() + session_expiry_interval,
            );
        }
    }
}
