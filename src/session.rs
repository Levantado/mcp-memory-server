use crate::models::MemoryScope;
use axum::response::sse::Event;
use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;
use std::sync::{Arc, RwLock};

pub struct Session {
    pub project_id: String,
    pub scope: MemoryScope,
    pub sender: mpsc::Sender<Result<Event, axum::Error>>,
    pub protocol_version: Arc<RwLock<String>>,
}

pub struct SessionManager {
    sessions: DashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn create_session(
        &self,
        project_id: String,
        scope: MemoryScope,
    ) -> (String, mpsc::Receiver<Result<Event, axum::Error>>) {
        let session_id = Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel(100);

        self.sessions.insert(
            session_id.clone(),
            Session {
                project_id,
                scope,
                sender: tx,
                protocol_version: Arc::new(RwLock::new("2024-11-05".to_string())),
            },
        );

        (session_id, rx)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.get(session_id).map(|s| Session {
            project_id: s.project_id.clone(),
            scope: s.scope.clone(),
            sender: s.sender.clone(),
            protocol_version: Arc::clone(&s.protocol_version),
        })
    }

    #[allow(dead_code)]
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }
}
