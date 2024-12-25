use std::{collections::HashSet, sync::Arc};

use bytes::BytesMut;
use dashmap::DashMap;
use log::{debug, error};
use tokio::{sync::mpsc, time::Instant};

#[derive(Debug)]
pub(crate) enum BrokerEvent {
    Publish(Option<BytesMut>),
}

#[derive(Debug)]
struct SessionState {
    tx: mpsc::Sender<BrokerEvent>,
    subscriptions: HashSet<String>,
}

impl SessionState {
    fn new(tx: mpsc::Sender<BrokerEvent>) -> Self {
        Self { tx, subscriptions: HashSet::new() }
    }
}

#[derive(Debug, Clone)]

pub(crate) struct BrokerState {
    sessions: Arc<DashMap<String, SessionState>>,
}

impl BrokerState {
    pub(crate) fn new() -> Self {
        Self { sessions: Arc::new(DashMap::new()) }
    }

    pub(crate) fn save_session(
        &mut self,
        client_id: String,
        clean_start: bool,
    ) -> (bool, mpsc::Receiver<BrokerEvent>) {
        let (tx, rx) = mpsc::channel(32);

        match self.sessions.insert(client_id, SessionState::new(tx)) {
            // If a previous session existed, return true unless clean_start is true
            Some(_) => (!clean_start, rx),
            None => (false, rx),
        }
    }

    pub(crate) fn schedule_discard_session(&mut self, client_id: &str, expires_at: Instant) {
        debug!("Session will expires at: {expires_at:?}");

        let session = self.sessions.remove(client_id);
        if session.is_none() {
            error!("Session with client id {client_id} does not exist");
        }
    }

    pub(crate) fn subscribe(&mut self, topic_filter: String, client_id: &str) {
        if let Some(mut session) = self.sessions.get_mut(client_id) {
            session.subscriptions.insert(topic_filter);
        } else {
            error!("Session with client id {client_id} does not exist");
        }
    }

    pub(crate) fn publish(&self, topic: &str, payload: Option<BytesMut>) {
        for entry in self.sessions.iter() {
            if entry.subscriptions.contains(topic) {
                let tx = entry.tx.clone();
                let payload = payload.clone();

                tokio::spawn(async move {
                    if let Err(e) = tx.send(BrokerEvent::Publish(payload)).await {
                        error!("Failed to send publish event: {:?}", e);
                    }
                });
            }
        }
    }
}
