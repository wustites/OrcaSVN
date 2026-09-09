#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod svn;

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
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
async fn configure_svn_executable(executable: Option<String>) -> Result<String, String> {
    svn::configure_svn_executable(executable.as_deref())
        .await
        .map_err(|e| e.to_string())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StashUnversionedEntry {
    path: String,
    kind: String,
    content_base64: Option<String>,
}

// Entries are persisted in WebView local storage. Keep enough headroom for
// base64 expansion, existing patches, and storage-engine bookkeeping.
const MAX_UNVERSIONED_STASH_BYTES: u64 = 2 * 1024 * 1024;

fn collect_unversioned_entry(
    workspace: &Path,
    target: &Path,
    entries: &mut Vec<StashUnversionedEntry>,
    total_bytes: &mut u64,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(target).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "symbolic links are not supported: {}",
            target.display()
        ));
    }
    let relative = target.strip_prefix(workspace).map_err(|e| e.to_string())?;
    let path = relative.to_string_lossy().replace('\\', "/");
    if metadata.is_dir() {
        entries.push(StashUnversionedEntry {
            path,
            kind: "directory".to_string(),
            content_base64: None,
        });
        let mut children = fs::read_dir(target)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            collect_unversioned_entry(workspace, &child.path(), entries, total_bytes)?;
        }
    } else {
        *total_bytes = total_bytes.saturating_add(metadata.len());
        if *total_bytes > MAX_UNVERSIONED_STASH_BYTES {
            return Err("unversioned files exceed the 2 MB stash limit".to_string());
        }
        let content = fs::read(target).map_err(|e| e.to_string())?;
        entries.push(StashUnversionedEntry {
            path,
            kind: "file".to_string(),
            content_base64: Some(BASE64_STANDARD.encode(content)),
        });
    }
    Ok(())
}

#[tauri::command]
async fn read_unversioned_files(
    path: String,
    files: Vec<String>,
) -> Result<Vec<StashUnversionedEntry>, String> {
    let workspace = fs::canonicalize(&path).map_err(|e| format!("invalid workspace path: {e}"))?;
    let statuses = svn::status(&path).await.map_err(|e| e.to_string())?;
    let unversioned: std::collections::HashSet<String> = statuses
        .into_iter()
        .filter(|status| status.status_code == "unversioned")
        .map(|status| status.path.replace('\\', "/"))
        .collect();
    let mut entries = Vec::new();
    let mut total_bytes = 0;
    for file in files {
        let normalized = file.replace('\\', "/");
        if !unversioned.contains(&normalized) {
            return Err(format!("path is not unversioned: {file}"));
        }
        let target = resolve_workspace_child(&workspace, &file)?;
        let canonical = fs::canonicalize(&target).map_err(|e| e.to_string())?;
        if !canonical.starts_with(&workspace) {
            return Err(format!("refusing to read outside workspace: {file}"));
        }
        collect_unversioned_entry(&workspace, &canonical, &mut entries, &mut total_bytes)?;
    }
    Ok(entries)
}

fn validate_restore_target(workspace: &Path, target: &Path) -> Result<(), String> {
    let mut existing_parent = target
        .parent()
        .ok_or_else(|| "invalid restore path".to_string())?;
    while !existing_parent.exists() {
        existing_parent = existing_parent
            .parent()
            .ok_or_else(|| "invalid restore path".to_string())?;
    }
    let canonical_parent = fs::canonicalize(existing_parent).map_err(|e| e.to_string())?;
    if !canonical_parent.starts_with(workspace) {
        return Err(format!(
            "refusing to restore outside workspace: {}",
            target.display()
        ));
    }
    Ok(())
}

#[tauri::command]
async fn restore_unversioned_files(
    path: String,
    files: Vec<StashUnversionedEntry>,
    dry_run: bool,
) -> Result<(), String> {
    let workspace = fs::canonicalize(&path).map_err(|e| format!("invalid workspace path: {e}"))?;
    if files.is_empty() {
        return Ok(());
    }

    let mut resolved = Vec::with_capacity(files.len());
    for entry in &files {
        if entry.kind != "file" && entry.kind != "directory" {
            return Err(format!("invalid stored entry kind: {}", entry.kind));
        }
        if entry.kind == "file" && entry.content_base64.is_none() {
            return Err(format!("stored file has no content: {}", entry.path));
        }
        let target = resolve_workspace_child(&workspace, &entry.path)?;
        validate_restore_target(&workspace, &target)?;
        if target.exists() {
            return Err(format!(
                "cannot restore because the path already exists: {}",
                entry.path
            ));
        }
        resolved.push((entry, target));
    }
    if dry_run {
        return Ok(());
    }

    resolved.sort_by_key(|(_, target)| target.components().count());
    for (entry, target) in resolved {
        if entry.kind == "directory" {
            fs::create_dir_all(&target)
                .map_err(|e| format!("failed to restore directory '{}': {e}", entry.path))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let encoded = entry.content_base64.as_deref().unwrap_or_default();
            let content = BASE64_STANDARD
                .decode(encoded)
                .map_err(|e| format!("invalid stored content for '{}': {e}", entry.path))?;
            fs::write(&target, content)
                .map_err(|e| format!("failed to restore file '{}': {e}", entry.path))?;
        }
    }
    Ok(())
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GitignoreData {
    pub content: String,
    pub mtime: u64,
}

#[tauri::command]
async fn read_gitignore(path: String) -> Result<Option<GitignoreData>, String> {
    let gitignore_path = Path::new(&path).join(".gitignore");
    if !gitignore_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&gitignore_path).map_err(|e| e.to_string())?;
    let mtime = fs::metadata(&gitignore_path)
        .map_err(|e| e.to_string())?
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Ok(Some(GitignoreData { content, mtime }))
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
    let workspace = PathBuf::from(path);
    let mut errors = Vec::new();

    let mut windows_terminal = Command::new("wt");
    windows_terminal.args(["-d", path]);
    match spawn_command(windows_terminal) {
        Ok(()) => return Ok(()),
        Err(error) => errors.push(format!("wt: {error}")),
    }

    let mut pwsh = Command::new("pwsh");
    pwsh.arg("-NoExit").current_dir(&workspace);
    match spawn_command(pwsh) {
        Ok(()) => return Ok(()),
        Err(error) => errors.push(format!("pwsh: {error}")),
    }

    let mut powershell = Command::new("powershell");
    powershell.arg("-NoExit").current_dir(&workspace);
    match spawn_command(powershell) {
        Ok(()) => return Ok(()),
        Err(error) => errors.push(format!("powershell: {error}")),
    }

    let mut cmd = Command::new("cmd");
    cmd.arg("/K").current_dir(&workspace);
    match spawn_command(cmd) {
        Ok(()) => Ok(()),
        Err(error) => {
            errors.push(format!("cmd: {error}"));
            Err(format!("failed to open terminal: {}", errors.join("; ")))
        }
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SvnAuthUser {
    pub username: String,
    pub realm: String,
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
    author: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<Vec<SvnLogEntry>, String> {
    svn::log(
        &path,
        limit,
        start_rev,
        end_rev,
        keyword.as_deref(),
        author.as_deref(),
        date_from.as_deref(),
        date_to.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_current_user(path: String) -> Result<Option<SvnAuthUser>, String> {
    svn::current_user(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_info(path: String) -> Result<SvnInfo, String> {
    svn::info(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_local_revision(path: String) -> Result<u64, String> {
    svn::local_revision(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn svn_remote_info(path: String) -> Result<SvnInfo, String> {
    svn::remote_info(&path).await.map_err(|e| e.to_string())
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
async fn svn_apply_patch(
    workspace_path: String,
    patch: String,
    reverse: bool,
) -> Result<CommandResult, String> {
    svn::apply_patch(&workspace_path, &patch, reverse)
        .await
        .map(|output| CommandResult {
            success: true,
            output,
            error: None,
        })
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

#[cfg(test)]
mod unversioned_stash_tests {
    use super::{collect_unversioned_entry, restore_unversioned_files};
    use std::fs;

    #[test]
    fn captures_and_restores_binary_files_and_empty_directories() {
        let root = std::env::temp_dir().join(format!(
            "orcasvn-unversioned-stash-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("new-dir");
        fs::create_dir_all(source.join("empty")).unwrap();
        fs::write(source.join("binary.dat"), [0, 1, 127, 255]).unwrap();

        let workspace = fs::canonicalize(&root).unwrap();
        let canonical_source = fs::canonicalize(&source).unwrap();
        let mut entries = Vec::new();
        let mut total_bytes = 0;
        collect_unversioned_entry(
            &workspace,
            &canonical_source,
            &mut entries,
            &mut total_bytes,
        )
        .unwrap();
        assert_eq!(total_bytes, 4);
        assert!(entries.iter().any(|entry| entry.path == "new-dir/empty"));

        fs::remove_dir_all(&source).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime
            .block_on(restore_unversioned_files(
                root.to_string_lossy().into_owned(),
                entries.clone(),
                true,
            ))
            .unwrap();
        assert!(!source.exists());
        runtime
            .block_on(restore_unversioned_files(
                root.to_string_lossy().into_owned(),
                entries.clone(),
                false,
            ))
            .unwrap();
        assert_eq!(
            fs::read(source.join("binary.dat")).unwrap(),
            [0, 1, 127, 255]
        );
        assert!(source.join("empty").is_dir());
        assert!(runtime
            .block_on(restore_unversioned_files(
                root.to_string_lossy().into_owned(),
                entries,
                true,
            ))
            .is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            configure_svn_executable,
            svn_checkout,
            svn_update,
            svn_commit,
            svn_status,
            svn_log,
            svn_current_user,
            svn_info,
            svn_local_revision,
            svn_remote_info,
            svn_diff,
            svn_apply_patch,
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
            read_unversioned_files,
            restore_unversioned_files,
            read_gitignore,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
