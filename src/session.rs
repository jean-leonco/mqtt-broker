use std::time::Duration;

use tokio::time::Instant;

use crate::{
    broker_state::BrokerState,
    connection::{Connection, IncomingPacket, OutgoingPacket},
    packets::{
        conn_ack_packet::{ConnAckPacket, ConnectReasonCode},
        disconnect_packet::{DisconnectPacket, DisconnectReasonCode},
    },
};

pub(crate) struct Session {
    connection: Connection,
    keep_alive: Duration,
    broker_state: BrokerState,
}

impl Session {
    pub async fn handle_session(
        mut connection: Connection,
        mut broker_state: BrokerState,
    ) -> anyhow::Result<()> {
        let first_packet = connection.read_packet().await.unwrap();

        let (mut session, client_id, session_expiry_interval) = match first_packet {
            Some(IncomingPacket::Connect(packet)) => {
                let session_present =
                    broker_state.save_session(packet.client_id.clone(), packet.clean_start);

                let response = ConnAckPacket::builder()
                    .session_present(session_present)
                    .reason_code(ConnectReasonCode::Success)
                    .build();
                connection.write_packet(OutgoingPacket::ConnAck(response)).await.unwrap();

                (
                    Self {
                        connection,
                        broker_state,
                        keep_alive: Duration::from_secs(packet.keep_alive as u64),
                    },
                    packet.client_id,
                    packet.session_expiry_interval,
                )
            }
            Some(_) => {
                anyhow::bail!("First packet was not CONNECT");
            }
            None => {
                anyhow::bail!("No packet received");
            }
        };

        session.run().await;

        if let Some(session_expiry_interval) = session_expiry_interval {
            session.broker_state.schedule_discard_session(
                &client_id,
                Instant::now() + Duration::from_secs(session_expiry_interval as u64),
            );
        }

        Ok(())
    }

    async fn run(&mut self) {
        let mut last_activity = Instant::now();

        loop {
            let packet = self.connection.read_packet().await.unwrap();

            match packet {
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
    }
}
