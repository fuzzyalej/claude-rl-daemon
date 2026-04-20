use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
pub struct DaemonState {
    pub pending: HashMap<String, PendingResume>,
    pub completed: HashSet<String>,
}

impl DaemonState {
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        if let Ok(bytes) = std::fs::read(path) {
            Ok(serde_json::from_slice(&bytes).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    pub fn save_to_path(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

pub struct Scheduler {
    state_path: PathBuf,
    state: DaemonState,
}

impl Scheduler {
    pub fn new(state_path: PathBuf) -> Self {
        let state = DaemonState::load_from_path(&state_path).unwrap_or_default();
        Self { state_path, state }
    }

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
        self.state.save_to_path(&self.state_path)?;

        info!(session_id = id, reset_at = %event.reset_at, "scheduled resume");
        Ok(true)
    }

    pub fn mark_completed(&mut self, session_id: &str) {
        self.state.pending.remove(session_id);
        self.state.completed.insert(session_id.to_string());
        let _ = self.state.save_to_path(&self.state_path);
    }

    pub fn is_pending(&self, session_id: &str) -> bool {
        self.state.pending.contains_key(session_id)
    }

    pub fn all_pending(&self) -> Vec<PendingResume> {
        self.state.pending.values().cloned().collect()
    }
}
