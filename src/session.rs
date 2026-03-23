use crate::models::MemoryScope;
use axum::response::sse::Event;
use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub struct Session {
    pub project_id: String,
    pub scope: MemoryScope,
    pub sender: mpsc::Sender<Result<Event, axum::Error>>,
    pub protocol_version: Arc<RwLock<String>>,
    pub last_activity: Arc<RwLock<Instant>>,
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
                last_activity: Arc::new(RwLock::new(Instant::now())),
            },
        );

        (session_id, rx)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        if let Some(s) = self.sessions.get(session_id) {
            // Update last activity
            if let Ok(mut last) = s.last_activity.write() {
                *last = Instant::now();
            }
            Some(Session {
                project_id: s.project_id.clone(),
                scope: s.scope.clone(),
                sender: s.sender.clone(),
                protocol_version: Arc::clone(&s.protocol_version),
                last_activity: Arc::clone(&s.last_activity),
            })
        } else {
            None
        }
    }

    /// Removes sessions that haven't been active for longer than `max_idle`
    pub fn cleanup_inactive(&self, max_idle: Duration) -> usize {
        let now = Instant::now();
        let mut count = 0;
        self.sessions.retain(|id, session| {
            let last = session.last_activity.read().map(|t| *t).unwrap_or(now);
            if now.duration_since(last) > max_idle {
                tracing::info!("Cleaning up inactive session: {}", id);
                count += 1;
                false // remove
            } else {
                true // keep
            }
        });
        count
    }
}
