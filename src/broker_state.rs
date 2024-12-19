use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use log::debug;
use tokio::time::Instant;

#[derive(Debug)]
struct SessionState {}

impl SessionState {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]

pub(crate) struct BrokerState {
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
}

impl BrokerState {
    pub fn new() -> Self {
        Self { sessions: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn save_session(&mut self, client_id: String, clean_start: bool) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let value = sessions.insert(client_id, SessionState::new());
        if clean_start {
            return false;
        }

        return value.is_some();
    }

    pub fn schedule_discard_session(&mut self, client_id: &str, expires_at: Instant) {
        debug!("Session will expires at: {expires_at:?}");
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(client_id);
    }
}
