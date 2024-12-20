use broker_state::BrokerState;
use connection::Connection;
use log::{error, info};
use session::Session;
use tokio::{io, net::TcpListener};

pub(crate) mod broker_state;
pub(crate) mod codec;
pub(crate) mod connection;
pub(crate) mod constants;
pub(crate) mod packets;
pub(crate) mod session;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let broker_state = BrokerState::new();

    info!("Starting MQTT broker on 127.0.0.1:1883...");
    let listener = TcpListener::bind("127.0.0.1:1883").await?;
    info!("Broker is listening on 127.0.0.1:1883");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client connected: {}", addr);

        let broker_state = broker_state.clone();
        tokio::spawn(async move {
            if let Err(e) = Session::handle_session(Connection::new(stream), broker_state).await {
                error!("Error handling connection from {}: {:?}", addr, e);
            }
        });
    }
}
