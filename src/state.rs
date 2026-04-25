use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ViewerState {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub show_bones: bool,
    #[serde(default)]
    pub materials: HashMap<String, MaterialState>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MaterialState {
    pub base_color: [f32; 4],
    #[serde(default)]
    pub base_color_texture_path: Option<PathBuf>,
}

const STATE_FILE: &str = "viewer_state.json";

pub fn load() -> Option<ViewerState> {
    let path = state_path();
    let bytes = std::fs::read(&path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("ignoring corrupt state at {}: {e}", path.display());
            None
        }
    }
}

pub fn save(state: &ViewerState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = match serde_json::to_string_pretty(state) {
        Ok(s) => s,
        Err(e) => { eprintln!("failed to serialize state: {e}"); return }
    };
    if let Err(e) = std::fs::write(&path, json) {
        eprintln!("failed to save state to {}: {e}", path.display());
    }
}

fn state_path() -> PathBuf {
    if let Some(dir) = config_dir() {
        return dir.join(STATE_FILE);
    }
    PathBuf::from(STATE_FILE)
}

fn config_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|s| PathBuf::from(s).join("model_viewer"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|s| PathBuf::from(s).join("Library/Application Support/model_viewer"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".config")))
            .map(|p| p.join("model_viewer"))
    }
}
