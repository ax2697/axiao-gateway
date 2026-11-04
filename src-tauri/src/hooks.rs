use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::model::HookEvent;
use crate::store::data_dir;

pub fn hook_file() -> std::path::PathBuf {
    data_dir().join("hook-events.jsonl")
}

#[tauri::command]
pub fn append_hook(kind: String, detail: String) -> Result<HookEvent, String> {
    let ev = HookEvent {
        ts: chrono::Utc::now().to_rfc3339(),
        kind,
        detail,
    };
    let _ = fs::create_dir_all(data_dir());
    let line = serde_json::to_string(&ev).map_err(|e| e.to_string())?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(hook_file())
        .map_err(|e| e.to_string())?;
    writeln!(f, "{line}").map_err(|e| e.to_string())?;
    Ok(ev)
}

#[tauri::command]
pub fn list_hooks(limit: usize) -> Result<Vec<HookEvent>, String> {
    let p = hook_file();
    if !p.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let mut out: Vec<HookEvent> = text
        .lines()
        .filter_map(|l| serde_json::from_str::<HookEvent>(l).ok())
        .collect();
    if out.len() > limit {
        out = out.split_off(out.len() - limit);
    }
    out.reverse();
    Ok(out)
}
