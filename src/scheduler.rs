use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::detector::RateLimitEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingResume {
    pub session_id: String,
    pub reset_at: DateTime<Utc>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    pending: HashMap<String, PendingResume>,
    completed: HashSet<String>,
}

pub struct Scheduler {
    state_path: PathBuf,
    state: State,
}

impl Scheduler {
    pub fn new(state_path: PathBuf) -> Self {
        let state = Self::load_state(&state_path);
        Self { state_path, state }
    }

    fn load_state(path: &PathBuf) -> State {
        if let Ok(bytes) = std::fs::read(path) {
            serde_json::from_slice(&bytes).unwrap_or_default()
        } else {
            State::default()
        }
    }

    fn save_state(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.state)?;
        std::fs::write(&self.state_path, bytes)?;
        Ok(())
    }

    /// Returns true if the event was newly scheduled, false if duplicate.
    pub async fn try_schedule(&mut self, event: RateLimitEvent) -> anyhow::Result<bool> {
        let id = &event.session_id;

        if self.state.completed.contains(id) || self.state.pending.contains_key(id) {
            warn!(session_id = id, "skipping duplicate schedule");
            return Ok(false);
        }

        let resume = PendingResume {
            session_id: id.clone(),
            reset_at: event.reset_at,
            cwd: event.cwd,
        };

        self.state.pending.insert(id.clone(), resume);
        self.save_state()?;

        info!(session_id = id, reset_at = %event.reset_at, "scheduled resume");
        Ok(true)
    }

    pub fn mark_completed(&mut self, session_id: &str) {
        self.state.pending.remove(session_id);
        self.state.completed.insert(session_id.to_string());
        let _ = self.save_state();
    }

    pub fn is_pending(&self, session_id: &str) -> bool {
        self.state.pending.contains_key(session_id)
    }

    pub fn all_pending(&self) -> Vec<PendingResume> {
        self.state.pending.values().cloned().collect()
    }
}
