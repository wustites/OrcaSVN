mod svn;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SvnCommand {
    pub cmd: String,
    pub args: Vec<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SvnStatus {
    pub path: String,
    pub status: String,
    pub status_code: String,
    pub prop_status: String,
    pub locked: bool,
    pub history: bool,
    pub switched: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SvnLogEntry {
    pub revision: u64,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SvnInfo {
    pub path: String,
    pub url: String,
    pub repository_root: String,
    pub revision: u64,
    pub node_kind: String,
    pub schedule: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResult {
    pub path: String,
    pub diff: String,
    pub old_revision: u64,
    pub new_revision: u64,
}

#[tauri::command]
async fn svn_checkout(
    url: String,
    path: String,
    revision: Option<u64>,
) -> Result<CommandResult, String> {
    svn::checkout(&url, &path, revision).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_update(path: String, revision: Option<u64>) -> Result<CommandResult, String> {
    svn::update(&path, revision).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_commit(
    path: String,
    message: String,
    files: Option<Vec<String>>,
) -> Result<CommandResult, String> {
    svn::commit(&path, &message, files.as_deref()).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_status(path: String) -> Result<Vec<SvnStatus>, String> {
    eprintln!("svn_status called with path: {}", path);
    match svn::status(&path).await {
        Ok(status) => {
            eprintln!("svn_status success: {} items", status.len());
            Ok(status)
        }
        Err(e) => {
            eprintln!("svn_status error: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn svn_log(
    path: String,
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
) -> Result<Vec<SvnLogEntry>, String> {
    svn::log(&path, limit, start_rev, end_rev).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_info(path: String) -> Result<SvnInfo, String> {
    eprintln!("svn_info called with path: {}", path);
    match svn::info(&path).await {
        Ok(info) => {
            eprintln!("svn_info success: {:?}", info);
            Ok(info)
        }
        Err(e) => {
            eprintln!("svn_info error: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn svn_diff(
    path: String,
    old_rev: Option<u64>,
    new_rev: Option<u64>,
) -> Result<DiffResult, String> {
    svn::diff(&path, old_rev, new_rev).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_blame(path: String) -> Result<Vec<svn::BlameLine>, String> {
    svn::blame(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_add(path: String, files: Vec<String>) -> Result<CommandResult, String> {
    svn::add(&path, &files).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_delete(path: String, files: Vec<String>) -> Result<CommandResult, String> {
    svn::delete(&path, &files).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_revert(path: String, files: Vec<String>) -> Result<CommandResult, String> {
    svn::revert(&path, &files).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_resolve(path: String, files: Vec<String>, strategy: String) -> Result<CommandResult, String> {
    svn::resolve(&path, &files, &strategy).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_cleanup(path: String) -> Result<CommandResult, String> {
    svn::cleanup(&path).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_switch(path: String, url: String) -> Result<CommandResult, String> {
    svn::switch_cmd(&path, &url).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_merge(path: String, source: String, rev_start: u64, rev_end: u64) -> Result<CommandResult, String> {
    svn::merge(&path, &source, rev_start, rev_end).await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn select_directory() -> Result<Option<String>, String> {
    Ok(None)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            svn_checkout,
            svn_update,
            svn_commit,
            svn_status,
            svn_log,
            svn_info,
            svn_diff,
            svn_blame,
            svn_add,
            svn_delete,
            svn_revert,
            svn_resolve,
            svn_cleanup,
            svn_switch,
            svn_merge,
            select_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
