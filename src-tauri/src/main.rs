#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod svn;

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum OpenTarget {
    Explorer,
    Vscode,
    Terminal,
}

#[tauri::command]
async fn open_workspace_target(path: String, target: OpenTarget) -> Result<(), String> {
    match target {
        OpenTarget::Explorer => {
            let mut command = Command::new("explorer");
            command.arg(&path);
            spawn_command(command)
        }
        OpenTarget::Vscode => {
            let mut code_command = Command::new("code");
            code_command.arg(&path);
            if spawn_command(code_command).is_ok() {
                return Ok(());
            }

            for executable in vscode_candidates() {
                let mut command = Command::new(executable);
                command.arg(&path);
                if spawn_command(command).is_ok() {
                    return Ok(());
                }
            }

            Err("failed to open workspace: VS Code executable was not found".to_string())
        }
        OpenTarget::Terminal => open_terminal(&path),
    }
}

fn spawn_command(mut command: Command) -> Result<(), String> {
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to open workspace: {e}"))
}

#[tauri::command]
async fn delete_unversioned(path: String, files: Vec<String>) -> Result<CommandResult, String> {
    let workspace = fs::canonicalize(&path).map_err(|e| format!("invalid workspace path: {e}"))?;
    let mut removed = Vec::new();

    for file in files {
        let target = resolve_workspace_child(&workspace, &file)?;
        if !target.exists() {
            return Err(format!("file does not exist: {file}"));
        }

        let canonical_target = fs::canonicalize(&target)
            .map_err(|e| format!("failed to resolve file path '{file}': {e}"))?;
        if !canonical_target.starts_with(&workspace) {
            return Err(format!("refusing to delete outside workspace: {file}"));
        }

        if canonical_target.is_dir() {
            fs::remove_dir_all(&canonical_target)
                .map_err(|e| format!("failed to delete directory '{file}': {e}"))?;
        } else {
            fs::remove_file(&canonical_target)
                .map_err(|e| format!("failed to delete file '{file}': {e}"))?;
        }
        removed.push(file);
    }

    Ok(CommandResult {
        success: true,
        output: removed.join("\n"),
        error: None,
    })
}

fn resolve_workspace_child(workspace: &Path, file: &str) -> Result<PathBuf, String> {
    let relative = Path::new(file);
    if relative.is_absolute() {
        return Err(format!("absolute paths are not allowed: {file}"));
    }

    let mut target = workspace.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => target.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("invalid path outside workspace: {file}"));
            }
        }
    }

    Ok(target)
}

fn vscode_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        candidates
            .push(PathBuf::from(&local_app_data).join("Programs\\Microsoft VS Code\\Code.exe"));
        candidates.push(
            PathBuf::from(&local_app_data)
                .join("Programs\\Microsoft VS Code Insiders\\Code - Insiders.exe"),
        );
    }

    if let Ok(program_files) = env::var("ProgramFiles") {
        candidates.push(PathBuf::from(&program_files).join("Microsoft VS Code\\Code.exe"));
        candidates.push(
            PathBuf::from(&program_files).join("Microsoft VS Code Insiders\\Code - Insiders.exe"),
        );
    }

    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(&program_files_x86).join("Microsoft VS Code\\Code.exe"));
        candidates.push(
            PathBuf::from(&program_files_x86)
                .join("Microsoft VS Code Insiders\\Code - Insiders.exe"),
        );
    }

    candidates
}

fn open_terminal(path: &str) -> Result<(), String> {
    let mut pwsh = Command::new("cmd");
    pwsh.args(["/C", "start", "", "pwsh", "-NoExit", "-Command"]);
    pwsh.arg(format!(
        "Set-Location -LiteralPath '{}'",
        escape_powershell_literal(path)
    ));
    if spawn_command(pwsh).is_ok() {
        return Ok(());
    }

    let mut powershell = Command::new("cmd");
    powershell.args(["/C", "start", "", "powershell", "-NoExit", "-Command"]);
    powershell.arg(format!(
        "Set-Location -LiteralPath '{}'",
        escape_powershell_literal(path)
    ));
    if spawn_command(powershell).is_ok() {
        return Ok(());
    }

    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "start", "", "cmd", "/K"]);
    cmd.arg(format!("cd /d \"{}\"", path));
    spawn_command(cmd)
}

fn escape_powershell_literal(path: &str) -> String {
    path.replace('\'', "''")
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
    pub changed_paths: Vec<SvnLogPath>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SvnLogPath {
    pub path: String,
    pub action: String,
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
    svn::checkout(&url, &path, revision)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_update(path: String, revision: Option<u64>) -> Result<CommandResult, String> {
    svn::update(&path, revision)
        .await
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
    svn::commit(&path, &message, files.as_deref())
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_status(path: String) -> Result<Vec<SvnStatus>, String> {
    svn::status(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_log(
    path: String,
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
    keyword: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<Vec<SvnLogEntry>, String> {
    svn::log(
        &path,
        limit,
        start_rev,
        end_rev,
        keyword.as_deref(),
        date_from.as_deref(),
        date_to.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_info(path: String) -> Result<SvnInfo, String> {
    svn::info(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_diff(
    workspace_path: String,
    file: String,
    old_rev: Option<u64>,
    new_rev: Option<u64>,
) -> Result<DiffResult, String> {
    svn::diff(&workspace_path, &file, old_rev, new_rev)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_blame(workspace_path: String, file: String) -> Result<Vec<svn::BlameLine>, String> {
    svn::blame(&workspace_path, &file)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_add(path: String, files: Vec<String>) -> Result<CommandResult, String> {
    svn::add(&path, &files)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_delete(path: String, files: Vec<String>) -> Result<CommandResult, String> {
    svn::delete(&path, &files)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_revert(path: String, files: Vec<String>) -> Result<CommandResult, String> {
    svn::revert(&path, &files)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_resolve(
    path: String,
    files: Vec<String>,
    strategy: String,
) -> Result<CommandResult, String> {
    svn::resolve(&path, &files, &strategy)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_cleanup(path: String) -> Result<CommandResult, String> {
    svn::cleanup(&path)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_switch(path: String, url: String) -> Result<CommandResult, String> {
    svn::switch_cmd(&path, &url)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_merge(
    path: String,
    source: String,
    rev_start: u64,
    rev_end: u64,
) -> Result<CommandResult, String> {
    svn::merge(&path, &source, rev_start, rev_end)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
        .map_err(|e| e.to_string())
}

fn main() {
    tracing_subscriber::fmt::init();

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
            open_workspace_target,
            delete_unversioned,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
