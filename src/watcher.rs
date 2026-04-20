use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::event::{EventKind, ModifyKind};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{error, info};

use crate::detector::detect_rate_limit;
use crate::scheduler::Scheduler;
use crate::session::{jsonl_path, SessionEntry};
use crate::tmux::spawn_resume;

pub async fn run() -> anyhow::Result<()> {
    let claude_dir = dirs::home_dir().expect("home dir").join(".claude");
    let sessions_dir = claude_dir.join("sessions");
    let state_path = dirs::home_dir()
        .unwrap()
        .join(".claude-daemon")
        .join("state.json");

    let scheduler = Arc::new(Mutex::new(Scheduler::new(state_path)));
    restore_pending_resumes(Arc::clone(&scheduler)).await;

    let (file_tx, mut file_rx) = mpsc::channel::<PathBuf>(64);
    // Channel for new JSONL paths to watch (avoids holding &mut watcher across awaits)
    let (watch_tx, mut watch_rx) = mpsc::channel::<PathBuf>(16);

    let mut watcher = RecommendedWatcher::new(
        {
            let tx = file_tx.clone();
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    if matches!(
                        event.kind,
                        EventKind::Modify(ModifyKind::Data(_)) | EventKind::Create(_)
                    ) {
                        for path in event.paths {
                            let _ = tx.blocking_send(path);
                        }
                    }
                }
            }
        },
        Config::default().with_poll_interval(Duration::from_millis(500)),
    )?;

    watcher.watch(&sessions_dir, RecursiveMode::NonRecursive)?;
    info!(dir = ?sessions_dir, "watching sessions directory");

    for jsonl in discover_active_jsonls(&claude_dir) {
        if jsonl.exists() {
            watcher.watch(&jsonl, RecursiveMode::NonRecursive)?;
            info!(path = ?jsonl, "watching active session JSONL");
        }
    }

    let offsets: Arc<Mutex<HashMap<PathBuf, u64>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        tokio::select! {
            Some(path) = watch_rx.recv() => {
                if path.exists() {
                    let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
                    info!(path = ?path, "watching new session JSONL");
                }
            }
            Some(path) = file_rx.recv() => {
                handle_change(
                    path,
                    &claude_dir,
                    Arc::clone(&scheduler),
                    Arc::clone(&offsets),
                    watch_tx.clone(),
                ).await;
            }
        }
    }
}

async fn handle_change(
    path: PathBuf,
    claude_dir: &PathBuf,
    scheduler: Arc<Mutex<Scheduler>>,
    offsets: Arc<Mutex<HashMap<PathBuf, u64>>>,
    watch_tx: mpsc::Sender<PathBuf>,
) {
    // New session PID file → queue its JSONL for watching
    if path.starts_with(claude_dir.join("sessions"))
        && path.extension().map_or(false, |e| e == "json")
    {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(entry) = serde_json::from_str::<SessionEntry>(&content) {
                let jsonl = jsonl_path(&entry);
                let _ = watch_tx.send(jsonl).await;
            }
        }
        return;
    }

    // JSONL changed → tail new lines and check for rate limit
    if path.extension().map_or(false, |e| e == "jsonl") {
        let new_lines = read_new_lines(&path, Arc::clone(&offsets)).await;
        for line in new_lines {
            if let Some(event) = detect_rate_limit(&line) {
                info!(session_id = event.session_id, reset_at = %event.reset_at, "rate limit detected");
                let mut sched = scheduler.lock().await;
                if let Ok(true) = sched.try_schedule(event.clone()).await {
                    drop(sched);
                    let sched_clone = Arc::clone(&scheduler);
                    let ev = event.clone();
                    tokio::spawn(async move {
                        let now = chrono::Utc::now();
                        let diff = ev.reset_at.signed_duration_since(now);
                        let delay = Duration::from_secs(diff.num_seconds().max(0) as u64);
                        info!(session_id = ev.session_id, delay_secs = delay.as_secs(), "waiting to resume");
                        sleep(delay).await;
                        let cwd = ev.cwd.unwrap_or_else(|| PathBuf::from("."));
                        match spawn_resume(&ev.session_id, &cwd) {
                            Ok(tmux_name) => {
                                let mut s = sched_clone.lock().await;
                                s.mark_completed(&ev.session_id);
                                info!(session_id = ev.session_id, tmux_session = tmux_name, "resume spawned");
                            }
                            Err(e) => {
                                error!(session_id = ev.session_id, error = %e, "failed to spawn resume")
                            }
                        }
                    });
                }
            }
        }
    }
}

async fn read_new_lines(
    path: &PathBuf,
    offsets: Arc<Mutex<HashMap<PathBuf, u64>>>,
) -> Vec<String> {
    let mut off = offsets.lock().await;
    let current_offset = *off.get(path).unwrap_or(&0);

    let Ok(mut file) = std::fs::File::open(path) else {
        return vec![];
    };
    let Ok(meta) = file.metadata() else {
        return vec![];
    };

    if meta.len() <= current_offset {
        return vec![];
    }

    let _ = file.seek(SeekFrom::Start(current_offset));
    let mut buf = String::new();
    let _ = file.read_to_string(&mut buf);
    off.insert(path.clone(), meta.len());

    buf.lines().map(String::from).collect()
}

fn discover_active_jsonls(claude_dir: &PathBuf) -> Vec<PathBuf> {
    let sessions_dir = claude_dir.join("sessions");
    let mut paths = vec![];

    let Ok(entries) = std::fs::read_dir(&sessions_dir) else {
        return paths;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().map_or(false, |e| e == "json") {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if let Ok(session) = serde_json::from_str::<SessionEntry>(&content) {
                    let jsonl = crate::session::jsonl_path(&session);
                    if jsonl.exists() {
                        paths.push(jsonl);
                    }
                }
            }
        }
    }
    paths
}

async fn restore_pending_resumes(scheduler: Arc<Mutex<Scheduler>>) {
    let pending = {
        let sched = scheduler.lock().await;
        sched.all_pending()
    };

    for resume in pending {
        let sched_clone = Arc::clone(&scheduler);
        let now = chrono::Utc::now();
        let diff = resume.reset_at.signed_duration_since(now);
        let delay = Duration::from_secs(diff.num_seconds().max(0) as u64);

        info!(
            session_id = resume.session_id,
            delay_secs = delay.as_secs(),
            "restoring pending resume from state file"
        );

        tokio::spawn(async move {
            sleep(delay).await;
            let cwd = resume.cwd.unwrap_or_else(|| PathBuf::from("."));
            match spawn_resume(&resume.session_id, &cwd) {
                Ok(tmux_name) => {
                    let mut s = sched_clone.lock().await;
                    s.mark_completed(&resume.session_id);
                    info!(
                        session_id = resume.session_id,
                        tmux_session = tmux_name,
                        "restored resume spawned"
                    );
                }
                Err(e) => {
                    error!(session_id = resume.session_id, error = %e, "restored resume failed")
                }
            }
        });
    }
}
