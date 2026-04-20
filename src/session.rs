use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    pub pid: u32,
    pub session_id: String,
    pub cwd: PathBuf,
    pub started_at: u64,
    pub version: String,
    pub kind: String,
    pub entrypoint: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SessionMessage {
    User {
        #[serde(flatten)]
        meta: MessageMeta,
    },
    Assistant {
        #[serde(flatten)]
        meta: MessageMeta,
    },
    System {
        subtype: Option<String>,
        content: Option<String>,
        #[serde(flatten)]
        meta: MessageMeta,
    },
    #[serde(rename = "last-prompt")]
    LastPrompt {
        #[serde(rename = "lastPrompt")]
        last_prompt: Option<String>,
        #[serde(rename = "sessionId")]
        session_id: Option<String>,
    },
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot {
        #[serde(flatten)]
        meta: MessageMeta,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageMeta {
    pub uuid: Option<String>,
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<PathBuf>,
}

/// Converts an absolute cwd path to the Claude project directory key.
/// "/Users/aan/Code/oje" → "-Users-aan-Code-oje"
pub fn cwd_to_project_key(cwd: &PathBuf) -> String {
    cwd.to_string_lossy().replace('/', "-")
}

/// Resolves the JSONL path for a given session entry.
pub fn jsonl_path(entry: &SessionEntry) -> PathBuf {
    dirs::home_dir()
        .expect("home dir")
        .join(".claude")
        .join("projects")
        .join(cwd_to_project_key(&entry.cwd))
        .join(format!("{}.jsonl", entry.session_id))
}
