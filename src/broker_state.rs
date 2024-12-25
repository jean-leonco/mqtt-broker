use std::{collections::HashSet, sync::Arc, time::Duration};

use bytes::Bytes;
use dashmap::DashMap;
use log::{debug, error};
use tokio::{sync::mpsc, task::JoinSet, time::Instant};

#[derive(Debug)]
pub(crate) enum BrokerEvent {
    Publish(String, Option<Bytes>),
}

#[derive(Debug)]
struct SessionState {
    tx: mpsc::Sender<BrokerEvent>,
    subscriptions: HashSet<String>,
    expiry_interval: Option<Duration>,
}

impl SessionState {
    fn new(tx: mpsc::Sender<BrokerEvent>, expiry_interval: Option<Duration>) -> Self {
        Self { tx, subscriptions: HashSet::new(), expiry_interval }
    }
}

struct Registry {
    sessions: DashMap<String, SessionState>,
}

#[derive(Clone)]
pub(crate) struct BrokerState {
    registry: Arc<Registry>,
    publish_event_tx: mpsc::Sender<PublishEvent>,
}

impl BrokerState {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);

        let registry = Arc::new(Registry { sessions: DashMap::new() });

        tokio::spawn(handle_publish_events(registry.clone(), rx));

        Self { registry, publish_event_tx: tx }
    }

    pub(crate) fn register_session(
        &mut self,
        client_id: String,
        clean_start: bool,
        expiry_interval: Option<Duration>,
    ) -> (bool, mpsc::Receiver<BrokerEvent>) {
        let (tx, rx) = mpsc::channel(32);

        match self.registry.sessions.insert(client_id, SessionState::new(tx, expiry_interval)) {
            // If a previous session existed, return true unless clean_start is true
            Some(_) => (!clean_start, rx),
            None => (false, rx),
        }
    }

    pub(crate) fn discard_session(&mut self, client_id: &str) {
        match self.registry.sessions.remove(client_id) {
            Some((_, session)) => {
                if let Some(expiry_interval) = session.expiry_interval {
                    let expires_at = Instant::now() + expiry_interval;
                    debug!("Session for client {client_id} will expires at: {:?}", expires_at);
                } else {
                    debug!("Session for client {client_id} does not expire");
                }
            }
            None => {
                error!("Session for client {client_id} does not exist");
            }
        }
    }

    pub(crate) fn subscribe(&mut self, topic_filter: String, client_id: &str) {
        if let Some(mut session) = self.registry.sessions.get_mut(client_id) {
            session.subscriptions.insert(topic_filter);
        } else {
            error!("Session for client {client_id} does not exist");
        }
    }

    pub(crate) async fn publish(
        &self,
        topic: String,
        payload: Option<Bytes>,
    ) -> Result<(), mpsc::error::SendError<PublishEvent>> {
        self.publish_event_tx.send(PublishEvent { topic, payload }).await
    }
}

pub(crate) struct PublishEvent {
    topic: String,
    payload: Option<Bytes>,
}

async fn handle_publish_events(
    registry: Arc<Registry>,
    mut rx: mpsc::Receiver<PublishEvent>,
) -> tokio::io::Result<()> {
    // TODO: Batch events
    while let Some(event) = rx.recv().await {
        let mut set = JoinSet::new();

        for session in registry.sessions.iter() {
            if session.subscriptions.contains(&event.topic) {
                let tx = session.tx.clone();
                let topic = event.topic.to_string();
                let payload = event.payload.clone();

                set.spawn(async move {
                    if let Err(e) = tx.send(BrokerEvent::Publish(topic, payload)).await {
                        error!("Failed to send publish event: {:?}", e);
                    }
                });
            }
        }

        set.join_all().await;
    }

    Ok(())
}
