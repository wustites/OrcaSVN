use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lang: Lang,
    pub theme: Theme,
    pub svn_path: String,
    pub log_limit: u32,
    pub workspace: Option<String>,
    pub recent: Vec<String>,
    pub zoom: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: Lang::default(),
            theme: Theme::default(),
            svn_path: String::new(),
            log_limit: 100,
            workspace: None,
            recent: Vec::new(),
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub id: String,
    pub name: String,
    pub workspace: String,
    pub created_at: String,
    pub patch: String,
    pub files: Vec<String>,
}

fn dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("", "", "OrcaSVN")
}

pub fn cache_dir() -> Option<PathBuf> {
    dirs().map(|d| d.cache_dir().join("workspace-status"))
}

fn config_file(name: &str) -> Option<PathBuf> {
    dirs().map(|d| d.config_dir().join(name))
}

fn load<T: for<'de> Deserialize<'de> + Default>(name: &str) -> T {
    config_file(name)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save<T: Serialize>(name: &str, value: &T) {
    let Some(path) = config_file(name) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(value) {
        Ok(text) => {
            if let Err(error) = fs::write(&path, text) {
                tracing::warn!(%error, "could not save {name}");
            }
        }
        Err(error) => tracing::warn!(%error, "could not serialize {name}"),
    }
}

impl Settings {
    pub fn load() -> Self {
        load("settings.json")
    }

    pub fn save(&self) {
        save("settings.json", self);
    }

    pub fn remember(&mut self, workspace: &str) {
        self.recent.retain(|path| path != workspace);
        self.recent.insert(0, workspace.to_string());
        self.recent.truncate(10);
        self.workspace = Some(workspace.to_string());
        self.save();
    }
}

pub fn load_stashes() -> Vec<StashEntry> {
    load("stashes.json")
}

pub fn save_stashes(stashes: &[StashEntry]) {
    save("stashes.json", &stashes);
}
